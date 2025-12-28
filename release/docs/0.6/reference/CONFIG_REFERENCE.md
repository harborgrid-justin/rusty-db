# RustyDB v0.6.0 - Configuration Quick Reference

**Version**: 0.6.0 | **Updated**: December 28, 2025

---

## Default Configuration

```toml
# Location: /etc/rusty-db/config.toml (production)
# Location: ./config.toml (development)

[server]
host = "127.0.0.1"
port = 5432
api_port = 8080
max_connections = 100

[storage]
data_dir = "/var/lib/rusty-db/data"
wal_dir = "/var/lib/rusty-db/wal"
page_size = 4096                    # 4 KB
buffer_pool_size = 1000             # pages (~4 MB)

[logging]
level = "info"                      # debug|info|warn|error
format = "json"                     # json|text
output = "stdout"                   # stdout|file

[security]
enable_ssl = false
ssl_cert = ""
ssl_key = ""
enable_auth = false
```

---

## View Current Configuration

```bash
# Using REST API
curl http://localhost:8080/api/v1/admin/config | jq

# Output
{
  "settings": {
    "buffer_pool_size": 1024,
    "max_connections": 1000,
    "data_directory": "/var/lib/rusty-db/data",
    "wal_directory": "/var/lib/rusty-db/wal"
  }
}
```

---

## Configuration Sections

### Server Configuration

```toml
[server]
# Network binding
host = "0.0.0.0"              # Listen on all interfaces (production)
host = "127.0.0.1"            # Localhost only (development)

# Ports
port = 5432                   # PostgreSQL protocol port
api_port = 8080               # REST/GraphQL API port

# Connection limits
max_connections = 1000        # Max concurrent connections
connection_timeout = 30       # Seconds

# Performance
worker_threads = 8            # Tokio worker threads (default: CPU cores)
```

**Apply Changes**: Restart server
```bash
sudo systemctl restart rustydb
```

---

### Storage Configuration

```toml
[storage]
# Directories
data_dir = "/var/lib/rusty-db/data"
wal_dir = "/var/lib/rusty-db/wal"
backup_dir = "/var/lib/rusty-db/backups"
temp_dir = "/var/lib/rusty-db/temp"

# Page size (DO NOT CHANGE after initialization)
page_size = 4096              # 4 KB (default)
# page_size = 8192            # 8 KB (larger pages)
# page_size = 16384           # 16 KB (very large)

# Buffer pool
buffer_pool_size = 10000      # pages (40 MB with 4KB pages)
# buffer_pool_size = 25000    # 100 MB
# buffer_pool_size = 250000   # 1 GB
# buffer_pool_size = 2500000  # 10 GB

# WAL settings
wal_buffer_size = 16          # MB
wal_sync_interval = 100       # ms
wal_compression = true

# Checkpoint settings
checkpoint_interval = 300     # seconds (5 minutes)
checkpoint_size = 1024        # MB
```

**Memory Calculation**:
```
Total Memory = buffer_pool_size × page_size
Example: 10,000 pages × 4,096 bytes = 40 MB
```

**Recommended Buffer Pool Sizes**:
- Development: 1,000 - 10,000 pages (4-40 MB)
- Small Production: 25,000 - 250,000 pages (100 MB - 1 GB)
- Medium Production: 250,000 - 1,000,000 pages (1-4 GB)
- Large Production: 1,000,000+ pages (4+ GB)

---

### Memory Configuration

```toml
[memory]
# Allocator settings
enable_slab_allocator = true
enable_arena_allocator = true
enable_large_object_allocator = true

# Slab allocator
slab_min_size = 16            # bytes
slab_max_size = 32768         # bytes (32 KB)
slab_magazine_size = 64       # objects per magazine

# Arena allocator
arena_initial_size = 65536    # bytes (64 KB)
arena_max_size = 16777216     # bytes (16 MB)

# Large object allocator
large_object_threshold = 262144  # bytes (256 KB)
enable_huge_pages = false     # Use 2MB/1GB pages

# Memory pressure
warning_threshold = 0.80      # 80% memory usage
critical_threshold = 0.90     # 90% memory usage
emergency_threshold = 0.95    # 95% memory usage
```

---

### Transaction Configuration

```toml
[transaction]
# MVCC settings
default_isolation_level = "READ_COMMITTED"
# Options: READ_UNCOMMITTED, READ_COMMITTED, REPEATABLE_READ, SERIALIZABLE

# Transaction limits
max_transaction_age = 3600    # seconds (1 hour)
max_locks_per_transaction = 10000

# Deadlock detection
deadlock_timeout = 1000       # ms
enable_deadlock_detection = true

# Snapshot retention
snapshot_retention_time = 300 # seconds (5 minutes)
max_snapshots = 1000
```

**Isolation Levels**:
- `READ_UNCOMMITTED`: Allows dirty reads (lowest isolation)
- `READ_COMMITTED`: Default, prevents dirty reads
- `REPEATABLE_READ`: Prevents non-repeatable reads
- `SERIALIZABLE`: Full serialization (highest isolation)

---

### Logging Configuration

```toml
[logging]
# Log level
level = "info"                # debug|info|warn|error

# Log format
format = "json"               # json|text
pretty_print = false          # Pretty-print JSON

# Log output
output = "stdout"             # stdout|file|syslog
file_path = "/var/log/rusty-db/server.log"

# Rotation
max_file_size = 100           # MB
max_files = 10                # Keep 10 files
compress_old_logs = true

# Log components
log_queries = true
log_slow_queries = true
slow_query_threshold = 1000   # ms
log_connections = true
log_transactions = false
```

**Log Levels**:
- `debug`: Verbose debugging information
- `info`: General information (default)
- `warn`: Warnings and above
- `error`: Errors only

---

### Security Configuration

```toml
[security]
# SSL/TLS
enable_ssl = true
ssl_cert = "/etc/rusty-db/ssl/server.crt"
ssl_key = "/etc/rusty-db/ssl/server.key"
ssl_ca = "/etc/rusty-db/ssl/ca.crt"

# Authentication
enable_auth = true
auth_method = "password"      # password|certificate|ldap

# Encryption
enable_tde = false            # Transparent Data Encryption
encryption_algorithm = "AES256"

# Network security
enable_rate_limiting = true
rate_limit_per_ip = 100       # requests per second
enable_ddos_protection = true

# Injection prevention
enable_sql_injection_prevention = true
enable_command_injection_prevention = true

# Audit logging
enable_audit_log = true
audit_log_path = "/var/log/rusty-db/audit.log"
audit_events = ["login", "logout", "create", "drop", "alter"]
```

---

### Performance Configuration

```toml
[performance]
# Query execution
enable_parallel_query = true
max_parallel_workers = 8
parallel_threshold = 1000     # rows

# Indexing
enable_index_advisor = true
auto_create_indexes = false

# Caching
enable_query_cache = true
query_cache_size = 100        # MB
cache_expiration = 300        # seconds

# SIMD
enable_simd = true            # Requires CPU support
simd_instruction_set = "AVX2" # AVX2|AVX512|NEON

# Compression
enable_compression = true
compression_algorithm = "LZ4" # LZ4|Snappy|Zstd
compression_level = 3         # 1-9
```

---

### Replication Configuration

```toml
[replication]
# Replication mode
enable_replication = false
replication_mode = "async"    # sync|async|semi-sync

# Master settings
is_master = true
listen_address = "0.0.0.0:5433"

# Replica settings
master_host = "192.168.1.100"
master_port = 5433
replication_user = "replicator"
replication_password = "secret"

# Lag tolerance
max_lag_time = 10             # seconds
max_lag_bytes = 10485760      # bytes (10 MB)
```

---

### Backup Configuration

```toml
[backup]
# Automatic backups
enable_auto_backup = true
backup_schedule = "0 2 * * *" # Cron format (2 AM daily)
backup_dir = "/var/lib/rusty-db/backups"

# Backup retention
keep_daily_backups = 7        # days
keep_weekly_backups = 4       # weeks
keep_monthly_backups = 12     # months

# Backup type
backup_type = "incremental"   # full|incremental|differential

# Compression
compress_backups = true
backup_compression = "Zstd"
```

---

### Monitoring Configuration

```toml
[monitoring]
# Metrics
enable_metrics = true
metrics_port = 9090
metrics_format = "prometheus" # prometheus|json

# Health checks
health_check_interval = 10    # seconds
enable_health_endpoint = true

# Profiling
enable_profiling = false
profiling_port = 6060

# Alerts
enable_alerts = true
alert_channels = ["email", "slack"]
email_recipients = ["admin@example.com"]
slack_webhook = "https://hooks.slack.com/..."
```

---

## Environment Variables

Override config file settings with environment variables:

```bash
# Server
export RUSTY_DB_HOST="0.0.0.0"
export RUSTY_DB_PORT=5432
export RUSTY_DB_API_PORT=8080

# Storage
export RUSTY_DB_DATA_DIR="/custom/data/path"
export RUSTY_DB_WAL_DIR="/custom/wal/path"
export RUSTY_DB_BUFFER_POOL_SIZE=10000

# Logging
export RUST_LOG=debug         # debug|info|warn|error
export RUST_LOG_FORMAT=json

# Security
export RUSTY_DB_ENABLE_SSL=true
export RUSTY_DB_SSL_CERT="/path/to/cert.pem"
export RUSTY_DB_SSL_KEY="/path/to/key.pem"
```

---

## Performance Tuning

### For OLTP (Transaction Processing)
```toml
[storage]
buffer_pool_size = 250000     # 1 GB
page_size = 4096              # 4 KB

[transaction]
default_isolation_level = "READ_COMMITTED"

[performance]
enable_parallel_query = false
enable_query_cache = true
query_cache_size = 200        # MB
```

### For OLAP (Analytics)
```toml
[storage]
buffer_pool_size = 2500000    # 10 GB
page_size = 8192              # 8 KB

[transaction]
default_isolation_level = "READ_UNCOMMITTED"

[performance]
enable_parallel_query = true
max_parallel_workers = 16
enable_simd = true
```

### For Mixed Workloads
```toml
[storage]
buffer_pool_size = 500000     # 2 GB
page_size = 4096              # 4 KB

[transaction]
default_isolation_level = "READ_COMMITTED"

[performance]
enable_parallel_query = true
max_parallel_workers = 8
enable_query_cache = true
query_cache_size = 100        # MB
```

---

## Configuration Examples

### Development Configuration
```toml
[server]
host = "127.0.0.1"
port = 5432
api_port = 8080
max_connections = 100

[storage]
data_dir = "./data"
wal_dir = "./wal"
buffer_pool_size = 1000       # ~4 MB

[logging]
level = "debug"
format = "text"
output = "stdout"

[security]
enable_ssl = false
enable_auth = false
```

### Production Configuration
```toml
[server]
host = "0.0.0.0"
port = 5432
api_port = 8080
max_connections = 1000
worker_threads = 16

[storage]
data_dir = "/var/lib/rusty-db/data"
wal_dir = "/var/lib/rusty-db/wal"
buffer_pool_size = 250000     # 1 GB
wal_compression = true

[logging]
level = "info"
format = "json"
output = "file"
file_path = "/var/log/rusty-db/server.log"
max_file_size = 100           # MB

[security]
enable_ssl = true
ssl_cert = "/etc/rusty-db/ssl/server.crt"
ssl_key = "/etc/rusty-db/ssl/server.key"
enable_auth = true
enable_audit_log = true

[backup]
enable_auto_backup = true
backup_schedule = "0 2 * * *"
keep_daily_backups = 7
```

---

## Apply Configuration Changes

### Development Mode
```bash
# 1. Edit config
nano config.toml

# 2. Restart server
# Stop with Ctrl+C, then restart
./builds/linux/rusty-db-server --config config.toml
```

### Production Mode
```bash
# 1. Edit config
sudo nano /etc/rusty-db/config.toml

# 2. Validate config (optional)
./builds/linux/rusty-db-server --config /etc/rusty-db/config.toml --validate

# 3. Restart service
sudo systemctl restart rustydb

# 4. Verify
sudo systemctl status rustydb
curl http://localhost:8080/api/v1/health
```

---

## Configuration Validation

```bash
# Validate configuration file
./builds/linux/rusty-db-server --config config.toml --validate

# Check for errors
echo $?  # 0 = valid, non-zero = errors

# View parsed configuration
./builds/linux/rusty-db-server --config config.toml --show-config
```

---

## Troubleshooting

### Configuration Not Applied
```bash
# Check config file location
./builds/linux/rusty-db-server --show-config-path

# Check for syntax errors
./builds/linux/rusty-db-server --config config.toml --validate

# Check systemd service config
sudo systemctl cat rustydb | grep ExecStart
```

### Buffer Pool Too Large
```
Error: Cannot allocate buffer pool: Out of memory
```

**Solution**: Reduce `buffer_pool_size` in config:
```toml
[storage]
buffer_pool_size = 10000      # Reduce from higher value
```

### Port Already in Use
```
Error: Address already in use (port 5432)
```

**Solution**: Change port or stop conflicting service:
```toml
[server]
port = 5433                   # Use different port
```

---

## Configuration Best Practices

1. **Start Small**: Begin with default config, tune based on workload
2. **Monitor First**: Monitor performance before making changes
3. **Change One Thing**: Modify one setting at a time
4. **Document Changes**: Comment your configuration changes
5. **Backup Config**: Keep backup of working configuration
6. **Test Changes**: Test in development before production
7. **Use Version Control**: Track config changes in git

---

**Configuration Reference** | RustyDB v0.6.0 | Enterprise Database Server
