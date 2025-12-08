# Connection Pool API Quick Reference

## Table of Contents
1. [Pool Creation](#pool-creation)
2. [Configuration](#configuration)
3. [Connection Management](#connection-management)
4. [Partitioning](#partitioning)
5. [Monitoring](#monitoring)
6. [Web Management API](#web-management-api)

## Pool Creation

### Basic Pool
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
    .min_size(10)
    .max_size(100)
    .initial_size(20)
    .acquire_timeout(Duration::from_secs(30))
    .max_lifetime(Duration::from_secs(3600))
    .statement_cache_size(100)
    .build()?;
```

## Configuration

### Pool Config Options
```rust
pub struct PoolConfig {
    pub min_size: usize,              // Minimum connections
    pub max_size: usize,              // Maximum connections
    pub initial_size: usize,          // Initial pool size
    pub acquire_timeout: Duration,    // Timeout for getting connection
    pub max_lifetime: Option<Duration>,      // Max connection age
    pub idle_timeout: Option<Duration>,      // Max idle time
    pub validate_on_acquire: bool,           // Validate before use
    pub validate_on_release: bool,           // Validate on return
    pub validation_timeout: Duration,        // Validation timeout
    pub max_wait_queue_size: usize,         // Max waiters
    pub creation_throttle: Option<u64>,     // Max creations/sec
    pub maintenance_interval: Duration,     // Background cleanup
    pub statement_cache_size: usize,        // Statement cache size
    pub leak_detection_threshold: Option<Duration>,
    pub fair_queue: bool,                   // FIFO vs priority
    pub enable_partitioning: bool,          // Enable partitions
}
```

## Connection Management

### Acquiring Connections
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

## Partitioning

### Creating Partitions
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
    // Custom logic
    Some(format!("partition_{}", request.user.as_ref()?))
});
let manager = PartitionManager::new(RoutingStrategy::Custom(custom_router));
```

### Partition Request
```rust
use rusty_db::pool::PartitionRequest;

let request = PartitionRequest {
    user: Some("alice".to_string()),
    application: Some("web_app".to_string()),
    service: Some("analytics".to_string()),
    tenant: Some("acme_corp".to_string()),
    session_id: Some("session_123".to_string()),
    metadata: HashMap::new(),
};

let partition_id = manager.route_request(&request);
```

## Monitoring

### Pool Statistics
```rust
let stats = pool.statistics();

println!("Connections created: {}", stats.connections_created);
println!("Connections active: {}", stats.active_connections);
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

### Usage Patterns
```rust
let patterns = stats.usage_patterns;
println!("Peak hour: {}", patterns.peak_hour);
println!("Acquisitions by hour: {:?}", patterns.acquisitions_by_hour);
```

### Efficiency Metrics
```rust
let efficiency = stats.efficiency_metrics;
println!("Pool utilization: {:.2}%", efficiency.pool_utilization * 100.0);
println!("Cache hit rate: {:.2}%", efficiency.cache_hit_rate * 100.0);
println!("Connection reuse: {:.2}%", efficiency.connection_reuse_rate * 100.0);
```

### Leak Detection
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

### Dashboard Provider
```rust
use rusty_db::pool::DashboardProvider;

let dashboard = DashboardProvider::new(
    pool_stats.clone(),
    Duration::from_secs(5)  // Refresh every 5 seconds
);

let data = dashboard.get_dashboard_data();
println!("Dashboard:");
println!("  Active: {}", data.active_connections);
println!("  Total: {}", data.total_connections);
println!("  Success: {:.2}%", data.success_rate * 100.0);
println!("  Efficiency: {:.2}%", data.pool_efficiency * 100.0);
```

### Metrics Export
```rust
use rusty_db::pool::{MonitoringExporter, ExportFormat};

// Export as JSON
let exporter = MonitoringExporter::new(pool_stats.clone(), ExportFormat::Json);
let json = exporter.export();

// Export as Prometheus
let exporter = MonitoringExporter::new(pool_stats.clone(), ExportFormat::Prometheus);
let prometheus = exporter.export();

// Export as CSV
let exporter = MonitoringExporter::new(pool_stats.clone(), ExportFormat::Csv);
let csv = exporter.export();
```

## Web Management API

### Pool Management
```rust
use rusty_db::pool::api;

// Get pool configuration
let config = api::get_pool_config(&pool);
println!("Min: {}, Max: {}", config.min_size, config.max_size);

// Get pool statistics
let stats = api::get_pool_statistics(&pool);
println!("Active connections: {}", stats.active_connections);

// Get pool size information
let size = api::get_pool_size(&pool);
println!("Total: {}, Idle: {}, Active: {}",
         size.total, size.idle, size.active);
```

### Queue Management
```rust
// Get wait queue statistics
let queue_stats = api::get_queue_statistics(&wait_queue);
println!("Queue length: {}", queue_stats.current_size);
println!("Total enqueued: {}", queue_stats.total_enqueued);
println!("Avg wait time: {:?}", queue_stats.average_wait_time);
```

### Partition Management
```rust
// Get statistics for all partitions
let partition_stats = api::get_partition_statistics(&partition_manager);
for (partition_id, stats) in partition_stats {
    println!("{}: {} acquired, {} released",
             partition_id, stats.connections_acquired, stats.connections_released);
}

// List all partitions
let partitions = api::list_partitions(&partition_manager);
println!("Partitions: {:?}", partitions);
```

### Dashboard and Monitoring
```rust
// Get dashboard data
let dashboard_data = api::get_dashboard_data(&dashboard_provider);
println!("Dashboard snapshot at {:?}", dashboard_data.timestamp);

// Get detected leaks
let leaks = api::get_detected_leaks(&leak_detector);
if !leaks.is_empty() {
    eprintln!("WARNING: {} connection leaks detected", leaks.len());
}

// Export metrics
let metrics = api::export_metrics(&exporter);
println!("{}", metrics);
```

## Lifecycle Management

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

### Recycling Strategies
```rust
use rusty_db::pool::RecyclingStrategy;

// Fast recycling (clear caches only)
let strategy = RecyclingStrategy::Fast;

// Checked recycling (full state reset)
let strategy = RecyclingStrategy::Checked;

// Replace connection
let strategy = RecyclingStrategy::Replace;

// Adaptive (choose based on state)
let strategy = RecyclingStrategy::Adaptive;
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

## Wait Queue Features

### Priority Queuing
```rust
use rusty_db::pool::QueuePriority;

// Enqueue with different priorities
wait_queue.enqueue_with_priority(QueuePriority::Critical).await?;
wait_queue.enqueue_with_priority(QueuePriority::High).await?;
wait_queue.enqueue_with_priority(QueuePriority::Normal).await?;
wait_queue.enqueue_with_priority(QueuePriority::Low).await?;
```

### Deadlock Detection
```rust
use rusty_db::pool::DeadlockDetector;

let detector = DeadlockDetector::new(true);  // Enable detection
let has_deadlock = detector.check_deadlock(&wait_queue);
let stats = detector.statistics();
println!("Deadlocks detected: {}", stats.deadlocks_detected);
```

### Starvation Prevention
```rust
use rusty_db::pool::StarvationPrevention;

let prevention = StarvationPrevention::new(Duration::from_secs(30));
prevention.check_and_boost(&wait_queue);
let stats = prevention.statistics();
println!("Priority boosts: {}", stats.boosted_count);
```

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

    // Create partitions for different tenant tiers
    partition_mgr.create_partition(
        "enterprise_tier".to_string(),
        PartitionType::ResourceGroup("Enterprise".to_string()),
        PartitionLimits {
            max_connections: 100,
            min_connections: 10,
            ..Default::default()
        },
    );

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

    // 4. Use in request handlers
    async fn handle_request(
        pool: &ConnectionPool<DbConnection>,
        tenant: &str,
    ) -> Result<()> {
        let conn = pool.acquire().await?;

        // Execute query
        conn.connection().execute("SELECT * FROM data").await?;

        // Connection automatically returned
        Ok(())
    }

    // 5. Monitor in background
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

## Best Practices

1. **Always use builder pattern** for configuration
2. **Enable monitoring** in production
3. **Set reasonable timeouts** to prevent hangs
4. **Use partitioning** for multi-tenant applications
5. **Monitor leak detection** regularly
6. **Export metrics** to your monitoring system
7. **Configure aging policies** based on workload
8. **Use fair queuing** for consistent latency
9. **Set resource limits** per partition
10. **Validate connections** in critical paths
