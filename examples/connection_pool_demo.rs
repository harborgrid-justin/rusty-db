//! # Connection Pool Demo
//!
//! This example demonstrates the comprehensive connection pooling features
//! of RustyDB, including elastic sizing, partitioning, monitoring, and more.

use rusty_db::pool::{
    ConnectionPool, PoolConfig, ConnectionFactory, PoolError,
    PartitionManager, RoutingStrategy, PartitionRequest, PartitionType,
    PartitionLimits, DashboardProvider, MonitoringExporter, ExportFormat,
    LeakDetector, AgingPolicy, RecyclingStrategy,
};
use rusty_db::Result;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;

/// Example connection type
#[derive(Debug, Clone)]
struct DatabaseConnection {
    id: String,
    connected: bool,
}

/// Example connection factory
struct ExampleConnectionFactory {
    connection_string: String,
}

#[async_trait]
impl ConnectionFactory<DatabaseConnection> for ExampleConnectionFactory {
    async fn create(&self) -> Result<DatabaseConnection> {
        // Simulate connection creation
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(DatabaseConnection {
            id: uuid::Uuid::new_v4().to_string(),
            connected: true,
        })
    }

    async fn validate(&self, connection: &DatabaseConnection) -> Result<bool> {
        // Simulate validation
        Ok(connection.connected)
    }

    async fn reset(&self, connection: &mut DatabaseConnection) -> Result<()> {
        // Simulate state reset
        Ok(())
    }

    async fn close(&self, connection: DatabaseConnection) -> Result<()> {
        // Simulate connection close
        drop(connection);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== RustyDB Connection Pool Demo ===\n");

    // Example 1: Basic pool with elastic sizing
    basic_pool_example().await?;

    // Example 2: Pool with partitioning
    partitioned_pool_example().await?;

    // Example 3: Monitoring and statistics
    monitoring_example().await?;

    Ok(())
}

/// Example 1: Basic pool with elastic sizing
async fn basic_pool_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Basic Pool with Elastic Sizing");
    println!("-------------------------------------------");

    // Configure the pool
    let config = PoolConfig::builder()
        .min_size(5)
        .max_size(50)
        .initial_size(10)
        .acquire_timeout(Duration::from_secs(30))
        .max_lifetime(Duration::from_secs(3600))
        .idle_timeout(Duration::from_secs(300))
        .statement_cache_size(100)
        .build()?;

    println!("Pool configured:");
    println!("  - Min size: {}", config.min_size);
    println!("  - Max size: {}", config.max_size);
    println!("  - Initial size: {}", config.initial_size);
    println!("  - Max lifetime: {:?}", config.max_lifetime);

    // Create factory
    let factory = Arc::new(ExampleConnectionFactory {
        connection_string: "rusty-db://localhost:5432/mydb".to_string(),
    });

    // Create pool
    let pool = ConnectionPool::new(config, factory).await
        .map_err(|e| format!("Failed to create pool: {}", e))?;

    println!("\nPool created successfully!");
    println!("  - Total connections: {}", pool.size());
    println!("  - Idle connections: {}", pool.idle_count());
    println!("  - Active connections: {}", pool.active_count());

    // Acquire and use connections
    println!("\nAcquiring connections...");
    let mut connections = Vec::new();

    for i in 0..5 {
        match pool.acquire().await {
            Ok(conn) => {
                println!("  [{}] Acquired connection {}", i, conn.id());
                connections.push(conn);
            }
            Err(e) => {
                eprintln!("  [{}] Failed to acquire: {}", i, e);
            }
        }
    }

    println!("\nPool state after acquiring:");
    println!("  - Total connections: {}", pool.size());
    println!("  - Idle connections: {}", pool.idle_count());
    println!("  - Active connections: {}", pool.active_count());

    // Return connections (happens automatically on drop)
    drop(connections);

    // Small delay to let connections return
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("\nPool state after releasing:");
    println!("  - Total connections: {}", pool.size());
    println!("  - Idle connections: {}", pool.idle_count());
    println!("  - Active connections: {}", pool.active_count());

    // Get statistics
    let stats = pool.statistics();
    println!("\nPool statistics:");
    println!("  - Connections created: {}", stats.connections_created);
    println!("  - Connections acquired: {}", stats.connections_acquired);
    println!("  - Acquire success rate: {:.2}%", stats.success_rate * 100.0);
    println!("  - Average acquire time: {:?}", stats.average_acquire_time);

    println!("\n");
    Ok(())
}

/// Example 2: Pool with partitioning
async fn partitioned_pool_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Pool with Partitioning");
    println!("----------------------------------");

    // Create partition manager
    let partition_manager = PartitionManager::<DatabaseConnection>::new(
        RoutingStrategy::TenantBased
    );

    // Create partitions for different tenants
    let tenant1_limits = PartitionLimits {
        max_connections: 20,
        min_connections: 2,
        ..Default::default()
    };

    let tenant2_limits = PartitionLimits {
        max_connections: 10,
        min_connections: 1,
        ..Default::default()
    };

    let partition1 = partition_manager.create_partition(
        "tenant_acme".to_string(),
        PartitionType::Tenant("ACME Corp".to_string()),
        tenant1_limits,
    );

    let partition2 = partition_manager.create_partition(
        "tenant_globex".to_string(),
        PartitionType::Tenant("Globex Inc".to_string()),
        tenant2_limits,
    );

    println!("Created partitions:");
    println!("  - tenant_acme (ACME Corp): max={}, min={}", 20, 2);
    println!("  - tenant_globex (Globex Inc): max={}, min={}", 10, 1);

    // Route requests
    let request1 = PartitionRequest {
        tenant: Some("ACME Corp".to_string()),
        ..Default::default()
    };

    let request2 = PartitionRequest {
        tenant: Some("Globex Inc".to_string()),
        ..Default::default()
    };

    if let Some(partition_id) = partition_manager.route_request(&request1) {
        println!("\nRequest 1 routed to: {}", partition_id);
    }

    if let Some(partition_id) = partition_manager.route_request(&request2) {
        println!("Request 2 routed to: {}", partition_id);
    }

    // List all partitions
    println!("\nAll partitions:");
    for partition_id in partition_manager.list_partitions() {
        println!("  - {}", partition_id);
    }

    println!("\n");
    Ok(())
}

/// Example 3: Monitoring and statistics
async fn monitoring_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Monitoring and Statistics");
    println!("-------------------------------------");

    // Create pool
    let config = PoolConfig::builder()
        .min_size(5)
        .max_size(20)
        .initial_size(10)
        .build()?;

    let factory = Arc::new(ExampleConnectionFactory {
        connection_string: "rusty-db://localhost:5432/mydb".to_string(),
    });

    let pool = ConnectionPool::new(config, factory).await
        .map_err(|e| format!("Failed to create pool: {}", e))?;

    // Simulate some activity
    println!("\nSimulating connection activity...");
    for i in 0..10 {
        if let Ok(conn) = pool.acquire().await {
            // Simulate work
            tokio::time::sleep(Duration::from_millis(50)).await;
            drop(conn);
        }
    }

    // Get detailed statistics
    let stats = pool.statistics();
    println!("\nDetailed Statistics:");
    println!("  Connection Metrics:");
    println!("    - Created: {}", stats.connections_created);
    println!("    - Destroyed: {}", stats.connections_destroyed);
    println!("    - Acquired: {}", stats.connections_acquired);
    println!("    - Released: {}", stats.connections_released);
    println!("    - Active: {}", stats.active_connections);

    println!("\n  Performance Metrics:");
    println!("    - Acquire attempts: {}", stats.acquire_attempts);
    println!("    - Acquire successes: {}", stats.acquire_successes);
    println!("    - Acquire failures: {}", stats.acquire_failures);
    println!("    - Success rate: {:.2}%", stats.success_rate * 100.0);
    println!("    - Avg acquire time: {:?}", stats.average_acquire_time);

    println!("\n  Wait Time Percentiles:");
    println!("    - p50: {:?}", stats.wait_time_histogram.percentiles.p50);
    println!("    - p95: {:?}", stats.wait_time_histogram.percentiles.p95);
    println!("    - p99: {:?}", stats.wait_time_histogram.percentiles.p99);

    println!("\n  Usage Patterns:");
    println!("    - Peak hour: {}", stats.usage_patterns.peak_hour);

    println!("\n  Efficiency Metrics:");
    println!("    - Pool utilization: {:.2}%", stats.efficiency_metrics.pool_utilization * 100.0);
    println!("    - Connection reuse rate: {:.2}%", stats.efficiency_metrics.connection_reuse_rate * 100.0);

    // Export metrics in different formats
    println!("\n  Exporting metrics...");

    // Export as JSON
    println!("\n  JSON Format (sample):");
    let json_stats = serde_json::to_string_pretty(&stats)?;
    println!("{}", &json_stats[..std::cmp::min(200, json_stats.len())]);
    println!("    ...");

    println!("\n");
    Ok(())
}
