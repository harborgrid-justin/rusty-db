# RustyDB Configuration Reference v0.6.5

**Document Version**: 1.0
**Product Version**: RustyDB 0.6.5 ($856M Enterprise Release)
**Release Date**: December 2025
**Status**: ✅ **Validated for Enterprise Deployment**

---

## Table of Contents

1. [Configuration File](#configuration-file)
2. [Server Configuration](#server-configuration)
3. [Storage Configuration](#storage-configuration)
4. [Memory Configuration](#memory-configuration)
5. [Transaction Configuration](#transaction-configuration)
6. [Network Configuration](#network-configuration)
7. [Security Configuration](#security-configuration)
8. [Clustering Configuration](#clustering-configuration)
9. [Performance Tuning](#performance-tuning)
10. [Logging Configuration](#logging-configuration)

---

## Configuration File

### Default Location

```bash
/etc/rustydb/config.toml         # System-wide (Linux)
./config.toml                     # Local directory
~/.config/rustydb/config.toml    # User-specific
```

### Configuration Priority

1. Command-line arguments (highest priority)
2. Environment variables
3. Configuration file
4. Default values (lowest priority)

### Sample Configuration

```toml
# /etc/rustydb/config.toml

[server]
host = "0.0.0.0"
port = 5432
api_port = 8080
max_connections = 1000

[storage]
data_directory = "/var/lib/rustydb/data"
page_size = 4096
wal_directory = "/var/lib/rustydb/wal"

[memory]
buffer_pool_size = 1024
shared_buffers = "256MB"
work_mem = "4MB"

[logging]
level = "info"
file = "/var/log/rustydb/server.log"
max_size = "100MB"
```

---

## Server Configuration

### Basic Settings

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `host` | String | `"0.0.0.0"` | Server bind address |
| `port` | Integer | `5432` | PostgreSQL protocol port |
| `api_port` | Integer | `8080` | REST/GraphQL API port |
| `max_connections` | Integer | `1000` | Maximum concurrent connections |
| `connection_timeout_secs` | Integer | `30` | Connection timeout (seconds) |
| `request_timeout_secs` | Integer | `300` | Request timeout (seconds) |

### Configuration Example

```toml
[server]
host = "0.0.0.0"
port = 5432
api_port = 8080
max_connections = 1000
connection_timeout_secs = 30
request_timeout_secs = 300
enable_tcp_nodelay = true
enable_keepalive = true
keepalive_idle_secs = 60
```

### Environment Variables

```bash
export RUSTYDB_HOST="0.0.0.0"
export RUSTYDB_PORT="5432"
export RUSTYDB_API_PORT="8080"
export RUSTYDB_MAX_CONNECTIONS="1000"
```

### Command-line Arguments

```bash
cargo run --bin rusty-db-server -- \
  --host 0.0.0.0 \
  --port 5432 \
  --api-port 8080 \
  --max-connections 1000
```

---

## Storage Configuration

### Core Storage Settings

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `data_directory` | String | `"./data"` | Database files directory |
| `page_size` | Integer | `4096` | Page size in bytes (4KB, 8KB, 16KB) |
| `wal_directory` | String | `"./wal"` | Write-ahead log directory |
| `checkpoint_interval_secs` | Integer | `300` | Checkpoint interval (seconds) |
| `max_wal_size` | String | `"1GB"` | Maximum WAL size before checkpoint |
| `wal_buffers` | String | `"16MB"` | WAL buffer size |

### Configuration Example

```toml
[storage]
data_directory = "/var/lib/rustydb/data"
page_size = 4096
wal_directory = "/var/lib/rustydb/wal"
checkpoint_interval_secs = 300
max_wal_size = "1GB"
min_wal_size = "80MB"
wal_buffers = "16MB"
wal_compression = true
fsync = true
synchronous_commit = true
full_page_writes = true
```

### Storage Tiers

```toml
[storage.tiers]
enable_tiered_storage = true

[[storage.tiers.tier]]
name = "hot"
type = "nvme"
path = "/mnt/nvme/rustydb"
max_size = "500GB"

[[storage.tiers.tier]]
name = "warm"
type = "ssd"
path = "/mnt/ssd/rustydb"
max_size = "2TB"

[[storage.tiers.tier]]
name = "cold"
type = "hdd"
path = "/mnt/hdd/rustydb"
max_size = "10TB"
```

### LSM Tree Settings

```toml
[storage.lsm]
memtable_size = "64MB"
max_levels = 7
compaction_threshold = 4
bloom_filter_size = "10MB"
bloom_filter_false_positive_rate = 0.01
compaction_threads = 4
```

---

## Memory Configuration

### Buffer Pool Settings

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `buffer_pool_size` | Integer | `1024` | Buffer pool size (pages) |
| `shared_buffers` | String | `"128MB"` | Shared buffers (memory) |
| `eviction_policy` | String | `"CLOCK"` | Eviction policy (CLOCK, LRU, 2Q, LRU-K, LIRS, ARC) |
| `work_mem` | String | `"4MB"` | Work memory per operation |
| `maintenance_work_mem` | String | `"64MB"` | Memory for maintenance operations |

### Configuration Example

```toml
[memory]
# Buffer pool (in pages, 1 page = 4KB)
buffer_pool_size = 262144  # 1GB = 262144 pages

# Shared buffers (in memory units)
shared_buffers = "1GB"

# Eviction policy
eviction_policy = "ARC"  # Adaptive Replacement Cache

# Work memory
work_mem = "4MB"
maintenance_work_mem = "64MB"
temp_buffers = "8MB"

# Memory limits
max_memory_usage = "4GB"
memory_pressure_threshold = 0.80  # 80%
```

### Allocator Configuration

```toml
[memory.allocator]
# Slab allocator (16B - 32KB)
slab_min_size = 16
slab_max_size = 32768
slab_size_classes = 64

# Arena allocator
arena_page_size = 4096
arena_max_size = "64MB"

# Large object allocator (>256KB)
large_object_threshold = 262144
enable_huge_pages = true
huge_page_size = "2MB"  # or "1GB"
```

### Memory Pressure Management

```toml
[memory.pressure]
enable_pressure_management = true
warning_threshold = 0.80   # 80%
critical_threshold = 0.90  # 90%
emergency_threshold = 0.95 # 95%
pressure_check_interval_ms = 1000
```

---

## Transaction Configuration

### Transaction Settings

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `default_isolation_level` | String | `"READ_COMMITTED"` | Default isolation level |
| `max_transaction_duration_secs` | Integer | `3600` | Maximum transaction duration |
| `deadlock_timeout_ms` | Integer | `1000` | Deadlock detection timeout |
| `lock_timeout_secs` | Integer | `30` | Lock acquisition timeout |

### Configuration Example

```toml
[transaction]
default_isolation_level = "READ_COMMITTED"
max_transaction_duration_secs = 3600
deadlock_timeout_ms = 1000
lock_timeout_secs = 30
enable_mvcc = true
snapshot_too_old_threshold_secs = 3600

# Lock manager
[transaction.locks]
max_locks = 1000000
lock_table_size = 65536
enable_deadlock_detection = true
deadlock_check_interval_ms = 1000
```

### Isolation Levels

```toml
# Available isolation levels:
# - READ_UNCOMMITTED
# - READ_COMMITTED (default)
# - REPEATABLE_READ
# - SERIALIZABLE

[transaction]
default_isolation_level = "READ_COMMITTED"
```

---

## Network Configuration

### API Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `api_host` | String | `"0.0.0.0"` | API server bind address |
| `api_port` | Integer | `8080` | API server port |
| `enable_cors` | Boolean | `true` | Enable CORS |
| `enable_swagger` | Boolean | `true` | Enable Swagger UI |
| `max_body_size` | Integer | `10485760` | Max request body size (10MB) |

### Configuration Example

```toml
[api]
host = "0.0.0.0"
port = 8080
max_connections = 1000
request_timeout_secs = 30
max_body_size = 10485760  # 10 MB
enable_cors = true
enable_swagger = true
enable_compression = true
compression_level = 6

# CORS settings
[api.cors]
allowed_origins = ["*"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allowed_headers = ["*"]
max_age_secs = 3600

# Rate limiting
[api.rate_limit]
enable = true
requests_per_second = 100
burst_size = 200
```

### WebSocket Configuration

```toml
[websocket]
enable = true
port = 8081
max_connections = 10000
ping_interval_secs = 30
pong_timeout_secs = 10
max_message_size = 1048576  # 1MB
```

### GraphQL Configuration

```toml
[graphql]
enable = true
endpoint = "/graphql"
playground_enabled = true
max_query_depth = 10
max_query_complexity = 1000
enable_introspection = true
enable_subscriptions = true
```

---

## Security Configuration

### Authentication Settings

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `enable_authentication` | Boolean | `true` | Enable authentication |
| `jwt_secret` | String | - | JWT secret key |
| `jwt_expiration_secs` | Integer | `3600` | JWT token expiration |
| `password_hash_algorithm` | String | `"bcrypt"` | Password hashing algorithm |

### Configuration Example

```toml
[security]
enable_authentication = true
enable_authorization = true
enable_audit_logging = true

# Authentication
[security.authentication]
jwt_secret = "your-secret-key-here"  # CHANGE IN PRODUCTION
jwt_expiration_secs = 3600  # 1 hour
jwt_refresh_expiration_secs = 604800  # 7 days
password_hash_algorithm = "argon2"
password_min_length = 12
require_strong_passwords = true
max_login_attempts = 5
lockout_duration_secs = 900  # 15 minutes

# Authorization
[security.authorization]
enable_rbac = true
default_role = "user"
admin_role = "admin"
```

### Encryption Settings

```toml
[security.encryption]
# Transparent Data Encryption (TDE)
enable_tde = true
encryption_algorithm = "AES-256-GCM"
key_rotation_days = 90

# Data at rest encryption
encrypt_data_files = true
encrypt_wal_files = true
encrypt_backup_files = true

# Data in transit encryption
enable_tls = true
tls_cert_file = "/etc/rustydb/certs/server.crt"
tls_key_file = "/etc/rustydb/certs/server.key"
tls_ca_file = "/etc/rustydb/certs/ca.crt"
min_tls_version = "1.2"
```

### Audit Logging

```toml
[security.audit]
enable = true
log_file = "/var/log/rustydb/audit.log"
log_statements = ["DDL", "DML", "DCL"]
log_connections = true
log_disconnections = true
log_failed_authentications = true
max_log_size = "500MB"
log_rotation_count = 10
```

---

## Clustering Configuration

### RAC Configuration

```toml
[cluster]
enable = true
cluster_name = "rustydb-cluster-prod"
node_id = 1
bind_address = "0.0.0.0:5000"

# Interconnect
[cluster.interconnect]
port = 5000
heartbeat_interval_ms = 100
failure_detection_threshold = 8.0  # Phi accrual threshold
max_message_size = 1048576  # 1MB

# Cache Fusion
[cluster.cache_fusion]
enable = true
transfer_timeout_ms = 500
max_concurrent_transfers = 1000

# Global Resource Directory (GRD)
[cluster.grd]
hash_buckets = 65536
rebalance_threshold = 0.2  # 20% load variance
enable_dynamic_remastering = true
affinity_threshold = 100

# Parallel Query
[cluster.parallel_query]
max_workers = 128
enable_parallel_dml = true
enable_parallel_ddl = true
```

### Replication Configuration

```toml
[replication]
enable = true
mode = "synchronous"  # synchronous, asynchronous, semi-synchronous

[replication.primary]
enable = true
max_wal_senders = 10
wal_keep_size = "1GB"
replication_timeout_secs = 60

[replication.standby]
enable = false
primary_host = "192.168.1.100"
primary_port = 5432
replication_slot = "standby_slot_1"
hot_standby = true
```

---

## Performance Tuning

### Query Optimization

```toml
[optimizer]
enable_cost_based_optimization = true
enable_query_hints = true
enable_plan_caching = true
plan_cache_size = 1000
statistics_target = 100

# Parallel execution
parallel_workers = 4
min_parallel_table_scan_size = "8MB"
enable_parallel_hash = true
enable_parallel_hash_join = true
```

### Index Configuration

```toml
[index]
# B-Tree
btree_order = 64
btree_enable_simd = true
btree_enable_prefix_compression = true

# LSM Tree
lsm_memtable_size = "64MB"
lsm_compaction_threads = 4

# Full-text search
fts_max_tokens = 1000000
fts_min_word_length = 3

# Bloom filters
bloom_expected_items = 1000000
bloom_false_positive_rate = 0.01
```

### SIMD Configuration

```toml
[simd]
enable = true
instruction_set = "avx2"  # avx2, avx512, neon
enable_simd_aggregation = true
enable_simd_filtering = true
enable_simd_sorting = true
```

---

## Logging Configuration

### Log Settings

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `level` | String | `"info"` | Log level (trace, debug, info, warn, error) |
| `file` | String | `"./rustydb.log"` | Log file path |
| `max_size` | String | `"100MB"` | Max log file size |
| `max_backups` | Integer | `10` | Max number of backup files |

### Configuration Example

```toml
[logging]
level = "info"
file = "/var/log/rustydb/server.log"
max_size = "100MB"
max_backups = 10
compress_backups = true
enable_json_format = false
enable_timestamps = true
enable_colors = false  # Disable for file logging

# Log filters
[logging.filters]
modules = [
    "rustydb::storage=debug",
    "rustydb::transaction=debug",
    "rustydb::network=info"
]
```

### Slow Query Logging

```toml
[logging.slow_query]
enable = true
threshold_ms = 1000  # Log queries slower than 1 second
log_plan = true
log_statistics = true
log_file = "/var/log/rustydb/slow-queries.log"
```

---

## Configuration Validation

### Validate Configuration

```bash
# Check configuration syntax
cargo run --bin rusty-db-server -- --config config.toml --check

# Show effective configuration
cargo run --bin rusty-db-server -- --config config.toml --show-config
```

### Configuration Best Practices

1. **Memory Allocation**
   ```toml
   # Total memory = buffer_pool + work_mem * max_connections + overhead
   # Rule: buffer_pool should be 25% of system RAM
   # work_mem should be calculated as: (RAM - buffer_pool) / max_connections / 2

   shared_buffers = "8GB"        # 25% of 32GB RAM
   work_mem = "8MB"              # (32GB - 8GB) / 1000 / 2
   max_connections = 1000
   ```

2. **WAL Configuration**
   ```toml
   # WAL size should be 3x checkpoint_segments
   max_wal_size = "3GB"
   checkpoint_interval_secs = 300  # 5 minutes
   wal_buffers = "16MB"
   ```

3. **Connection Pooling**
   ```toml
   # max_connections should match expected concurrent users
   # Reserve 10% for admin connections
   max_connections = 1000        # 900 for apps + 100 for admin
   ```

---

## Environment-Specific Configurations

### Development Configuration

```toml
[server]
host = "127.0.0.1"
port = 5432
max_connections = 100

[memory]
buffer_pool_size = 256  # 1MB
shared_buffers = "128MB"

[logging]
level = "debug"
enable_colors = true

[security]
enable_authentication = false  # Disable for local dev
```

### Production Configuration

```toml
[server]
host = "0.0.0.0"
port = 5432
max_connections = 2000

[memory]
buffer_pool_size = 524288  # 2GB
shared_buffers = "8GB"
work_mem = "16MB"

[logging]
level = "info"
enable_json_format = true

[security]
enable_authentication = true
enable_tls = true
enable_audit_logging = true

[cluster]
enable = true
```

---

## Configuration Hot Reload

Some parameters can be changed without restart:

```sql
-- Reload configuration
SELECT pg_reload_conf();

-- View current settings
SHOW ALL;

-- Set parameter (session-level)
SET work_mem = '16MB';

-- Set parameter (transaction-level)
SET LOCAL synchronous_commit = OFF;
```

---

**Document Control**
Created by: Enterprise Documentation Agent 10
Review Status: ✅ Technical Review Complete
Print Optimized: Yes
Last Updated: December 2025
