# RustyDB API Reference

## Table of Contents
1. [Core Database API](#core-database-api)
2. [Query Execution API](#query-execution-api)
3. [Transaction API](#transaction-api)
4. [Storage API](#storage-api)
5. [Index API](#index-api)
6. [Security API](#security-api)
7. [Monitoring API](#monitoring-api)
8. [Backup & Recovery API](#backup--recovery-api)

---

## Core Database API

### Database Connection

```rust
use rusty_db::{Database, Config};

// Create database with default configuration
let db = Database::new("./data")?;

// Create with custom configuration
let config = Config {
    data_dir: "./data".to_string(),
    page_size: 4096,
    buffer_pool_size: 1000,
    port: 5432,
};
let db = Database::with_config(config)?;
```

### Database Operations

```rust
// Execute SQL query
let result = db.execute("SELECT * FROM users")?;

// Execute with parameters (prepared statement)
let result = db.execute_prepared(
    "SELECT * FROM users WHERE id = ?",
    vec!["123".to_string()]
)?;

// Begin transaction
let txn = db.begin_transaction()?;

// Execute in transaction
txn.execute("INSERT INTO users VALUES (1, 'Alice')")?;
txn.commit()?;
```

---

## Query Execution API

### Planner

```rust
use rusty_db::execution::{Planner, Optimizer};
use rusty_db::parser::SqlStatement;

let planner = Planner::new();
let optimizer = Optimizer::new();

// Parse SQL
let statement = parser.parse("SELECT * FROM users WHERE age > 21")?;

// Create execution plan
let plan = planner.plan(&statement)?;

// Optimize plan
let optimized_plan = optimizer.optimize(plan)?;
```

### Executor

```rust
use rusty_db::execution::Executor;
use rusty_db::catalog::Catalog;
use rusty_db::transaction::TransactionManager;

let catalog = Arc::new(Catalog::new()?);
let txn_manager = Arc::new(TransactionManager::new()?);

let executor = Executor::new(catalog, txn_manager);

// Execute plan
let result = executor.execute(&plan)?;

println!("Columns: {:?}", result.columns);
println!("Rows: {:?}", result.rows);
println!("Affected: {}", result.rows_affected);
```

### CTE Execution

```rust
use rusty_db::execution::cte::{CteContext, CteDefinition, RecursiveCteEvaluator};

let mut cte_context = CteContext::new();

// Register CTE
let cte = CteDefinition {
    name: "regional_sales".to_string(),
    columns: vec!["region".to_string(), "total".to_string()],
    query: Box::new(plan_node),
    recursive: false,
};
cte_context.register_cte(cte)?;

// Execute with CTE context
let result = executor.execute_with_ctes(&plan, &cte_context)?;
```

### Subquery Evaluation

```rust
use rusty_db::execution::subquery::{
    SubqueryExpr, SubqueryType, ExistsEvaluator, InEvaluator, ScalarSubqueryEvaluator
};

// EXISTS subquery
let exists_result = ExistsEvaluator::evaluate(&subquery_result, false);

// IN subquery
let in_result = InEvaluator::evaluate("value", &subquery_result, false)?;

// Scalar subquery
let scalar_value = ScalarSubqueryEvaluator::evaluate(&subquery_result)?;
```

---

## Transaction API

### Basic Transactions

```rust
use rusty_db::transaction::{TransactionManager, TransactionId};

let txn_mgr = TransactionManager::new()?;

// Begin transaction
let txn_id = txn_mgr.begin()?;

// Execute operations
// ...

// Commit
txn_mgr.commit(txn_id)?;

// Or rollback
txn_mgr.rollback(txn_id)?;
```

### Lock Management

```rust
use rusty_db::transaction::{LockMode, LockManager};

let lock_mgr = LockManager::new();

// Acquire shared lock
lock_mgr.acquire_lock(txn_id, "users", LockMode::Shared)?;

// Acquire exclusive lock
lock_mgr.acquire_lock(txn_id, "users", LockMode::Exclusive)?;

// Release locks
lock_mgr.release_locks(txn_id)?;
```

### MVCC (Multi-Version Concurrency Control)

```rust
use rusty_db::transaction::VersionStore;

let version_store = VersionStore::new();

// Create new version
version_store.create_version(txn_id, row_id, data)?;

// Read version
let version = version_store.read_version(txn_id, row_id)?;
```

---

## Storage API

### Page Management

```rust
use rusty_db::storage::{StorageEngine, PageId};

let mut storage = StorageEngine::new("./data", 4096, 1000)?;

// Allocate new page
let page = storage.new_page()?;
println!("Page ID: {}", page.id());

// Get existing page
let page = storage.get_page(page_id)?;

// Modify page
page.write_data(offset, &data)?;

// Flush page to disk
storage.flush_page(page_id)?;
```

### Buffer Pool

```rust
use rusty_db::storage::BufferPoolManager;

let buffer_pool = BufferPoolManager::new(1000, disk_manager);

// Fetch page (from cache or disk)
let page = buffer_pool.fetch_page(page_id)?;

// Mark page as dirty
page.mark_dirty();

// Flush all pages
buffer_pool.flush_all()?;
```

### Partitioning

```rust
use rusty_db::storage::partitioning::{
    PartitionManager, PartitionStrategy, RangePartition
};

let mut manager = PartitionManager::new();

// Create range partitioned table
let strategy = PartitionStrategy::Range {
    column: "created_at".to_string(),
    ranges: vec![
        RangePartition {
            name: "p_2023".to_string(),
            lower_bound: Some("2023-01-01".to_string()),
            upper_bound: Some("2024-01-01".to_string()),
        },
    ],
};

manager.create_partitioned_table("events".to_string(), strategy)?;

// Get partition for value
let partition = manager.get_partition_for_value("events", "2023-06-15")?;
```

### JSON Storage

```rust
use rusty_db::storage::json::{JsonData, JsonPath, JsonOperators};

// Parse JSON
let json = JsonData::from_str(r#"{"name": "Alice", "age": 30}"#)?;

// Extract field
let name = JsonPath::extract(&json, "$.name")?;

// Update field
let updated = JsonOperators::json_set(&json, "$.age", JsonData::from_str("31")?)?;

// Merge JSON objects
let merged = JsonOperators::json_merge(&json1, &json2)?;
```

---

## Index API

### B-Tree Index

```rust
use rusty_db::index::{BTreeIndex, IndexKey};

let index = BTreeIndex::new("idx_users_email".to_string());

// Insert
index.insert(IndexKey::String("alice@example.com".to_string()), 1)?;

// Search
let results = index.search(&IndexKey::String("alice@example.com".to_string()))?;

// Range search
let range_results = index.range_search(
    &IndexKey::Integer(1),
    &IndexKey::Integer(100)
)?;
```

### Hash Index

```rust
use rusty_db::index::HashIndex;

let index = HashIndex::new("idx_users_id".to_string());

// Insert
index.insert(IndexKey::Integer(1), 100)?;

// Search (O(1) average)
let results = index.search(&IndexKey::Integer(1))?;
```

### Full-Text Index

```rust
use rusty_db::index::fulltext::{FullTextIndex, QueryParser};

let mut index = FullTextIndex::new("articles".to_string(), "content".to_string());

// Index document
index.index_document(1, "Rust programming language".to_string())?;

// Search
let results = index.search("rust programming")?;
for result in results {
    println!("Doc {}: score = {}", result.doc_id, result.score);
}

// Phrase search
let phrase_results = index.search_phrase("programming language")?;

// Wildcard search
let wildcard_results = index.search_wildcard("prog*")?;
```

---

## Security API

### Authentication

```rust
use rusty_db::security::SecurityManager;

let security_mgr = SecurityManager::new();

// Create user
security_mgr.create_user(
    "alice".to_string(),
    "password123".to_string(),
    vec!["admin".to_string()]
)?;

// Authenticate
let session_id = security_mgr.authenticate("alice", "password123")?;

// Logout
security_mgr.logout(&session_id)?;
```

### Authorization

```rust
use rusty_db::security::Permission;

// Check permission
let has_permission = security_mgr.authorize(&session_id, Permission::CreateTable)?;

// Grant permission
security_mgr.grant_permission("alice", Permission::Select, Some("users"))?;

// Revoke permission
security_mgr.revoke_permission("alice", Permission::Select, Some("users"))?;
```

### Role-Based Access Control

```rust
use rusty_db::security::Role;

// Create role
security_mgr.create_role("developer".to_string(), vec![
    Permission::Select,
    Permission::Insert,
    Permission::Update,
])?;

// Assign role to user
security_mgr.assign_role("bob", "developer")?;
```

---

## Monitoring API

### Query Statistics

```rust
use rusty_db::monitoring::{MonitoringSystem, QueryStats};

let monitoring = MonitoringSystem::new();

// Record query execution
let stats = QueryStats {
    query_id: "q123".to_string(),
    query: "SELECT * FROM users".to_string(),
    execution_time_ms: 150,
    rows_affected: 1000,
    bytes_read: 4096000,
    bytes_written: 0,
    cache_hits: 100,
    cache_misses: 10,
    timestamp: SystemTime::now(),
};
monitoring.record_query(stats);

// Get slow queries
let slow_queries = monitoring.get_slow_queries();
for query in slow_queries {
    println!("Slow query: {} ({}ms)", query.query, query.execution_time_ms);
}

// Get metrics
let metrics = monitoring.get_metrics();
println!("QPS: {}", metrics.queries_per_second);
println!("Buffer pool hit rate: {}%", metrics.buffer_pool_hit_rate * 100.0);
```

### Performance Metrics

```rust
// Update metrics
monitoring.update_metrics(|metrics| {
    metrics.active_connections += 1;
    metrics.total_queries += 1;
    metrics.disk_reads += 100;
});

// Get current metrics
let metrics = monitoring.get_metrics();
println!("Active connections: {}", metrics.active_connections);
println!("Active transactions: {}", metrics.active_transactions);
```

---

## Backup & Recovery API

### Full Backup

```rust
use rusty_db::backup::{BackupManager, BackupType};

let backup_mgr = BackupManager::new(config)?;

// Create full backup
let metadata = backup_mgr.create_backup("./data", BackupType::Full)?;
println!("Backup ID: {}", metadata.backup_id);
println!("Size: {} bytes", metadata.size_bytes);

// List backups
let backups = backup_mgr.list_backups();
for backup in backups {
    println!("{}: {} bytes", backup.backup_id, backup.size_bytes);
}
```

### Incremental Backup

```rust
// Create incremental backup
let metadata = backup_mgr.create_backup("./data", BackupType::Incremental)?;
```

### Point-in-Time Recovery

```rust
// Restore from backup
backup_mgr.restore_backup(&backup_id, "./restore")?;
```

---

## Advanced APIs

### Parallel Execution

```rust
use rusty_db::execution::parallel::{ParallelExecutor, ParallelizationOptimizer};

let parallel_executor = ParallelExecutor::new(4)?;  // 4 worker threads

// Check if plan can be parallelized
if ParallelizationOptimizer::can_parallelize(&plan) {
    // Execute in parallel
    let result = parallel_executor.execute_parallel(&plan).await?;
}

// Estimate speedup
let speedup = ParallelizationOptimizer::estimate_speedup(&plan, 4);
println!("Expected speedup: {}x", speedup);
```

### Resource Management

```rust
use rusty_db::operations::resources::{
    ResourceManager, ResourceConfig, ConnectionPriority
};

let config = ResourceConfig {
    max_memory_bytes: 4_000_000_000,  // 4 GB
    max_cpu_percent: 80,
    max_io_bytes_per_sec: 100_000_000,  // 100 MB/s
    max_connections: 1000,
    default_query_timeout: Duration::from_secs(300),
};

let resource_mgr = ResourceManager::new(config);

// Allocate resources for query
let allocation = resource_mgr.allocate_query_resources(
    "query_123".to_string(),
    1_000_000  // 1 MB estimated memory
)?;

// Execute query
// ...

// Release resources
resource_mgr.release_query_resources(&allocation);

// Get resource stats
let stats = resource_mgr.get_stats();
println!("Memory used: {} / {}", stats.memory_used, stats.memory_total);
println!("CPU usage: {}%", stats.cpu_usage);
```

### Advanced Optimization

```rust
use rusty_db::execution::optimization::{
    PlanCache, StatisticsCollector, AdaptiveOptimizer
};

// Plan caching
let cache = PlanCache::new(100);  // Cache up to 100 plans
cache.put(query_hash, plan, Duration::from_secs(3600));

if let Some(cached_plan) = cache.get(&query_hash) {
    // Use cached plan
}

// Statistics collection
let stats_collector = StatisticsCollector::new();
stats_collector.collect_table_stats("users".to_string(), 1_000_000, 4_096_000_000);
stats_collector.collect_column_stats(
    "users".to_string(),
    "email".to_string(),
    950_000,  // distinct count
    50_000,   // null count
    None,
    None
);

// Adaptive optimization
let adaptive_optimizer = AdaptiveOptimizer::new();
adaptive_optimizer.record_execution(
    query_hash,
    &plan,
    150,   // execution time in ms
    1000   // rows returned
);

// Learn from execution history
adaptive_optimizer.learn_join_orders();

// Get join order suggestions
if let Some(order) = adaptive_optimizer.suggest_join_order(&tables) {
    // Use suggested order
}
```

---

## Error Handling

All API methods return `Result<T, DbError>`:

```rust
use rusty_db::error::DbError;

match db.execute(sql) {
    Ok(result) => {
        // Handle success
    }
    Err(DbError::NotFound(msg)) => {
        println!("Not found: {}", msg);
    }
    Err(DbError::InvalidInput(msg)) => {
        println!("Invalid input: {}", msg);
    }
    Err(DbError::Internal(msg)) => {
        println!("Internal error: {}", msg);
    }
    Err(e) => {
        println!("Error: {:?}", e);
    }
}
```

---

## Complete Example

```rust
use rusty_db::{Database, Config};
use rusty_db::security::Permission;
use rusty_db::monitoring::MonitoringSystem;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create database
    let config = Config::default();
    let db = Database::with_config(config)?;
    
    // Setup security
    let security = db.security_manager();
    security.create_user("admin".into(), "password".into(), vec!["admin".into()])?;
    let session = security.authenticate("admin", "password")?;
    
    // Create table
    security.authorize(&session, Permission::CreateTable)?;
    db.execute("CREATE TABLE users (id INT, name VARCHAR(100), email VARCHAR(100))")?;
    
    // Create full-text index
    db.execute("CREATE FULLTEXT INDEX idx_users ON users(name, email)")?;
    
    // Insert data
    db.execute("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com')")?;
    db.execute("INSERT INTO users VALUES (2, 'Bob', 'bob@example.com')")?;
    
    // Query with monitoring
    let monitoring = db.monitoring_system();
    let result = db.execute("SELECT * FROM users WHERE MATCH(name) AGAINST ('Alice')")?;
    
    println!("Found {} rows", result.rows.len());
    
    // Get performance metrics
    let metrics = monitoring.get_metrics();
    println!("QPS: {}", metrics.queries_per_second);
    
    Ok(())
}
```

---

This API reference provides comprehensive coverage of RustyDB's functionality with practical examples.
