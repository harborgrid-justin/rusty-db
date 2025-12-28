# RustyDB v0.6.0 - Configuration Reference

**Document Version**: 1.0
**Release**: v0.6.0
**Last Updated**: 2025-12-28
**Classification**: Enterprise Operations
**Target Audience**: Database Administrators, System Administrators

---

## Table of Contents

1. [Configuration Overview](#configuration-overview)
2. [Configuration File Format](#configuration-file-format)
3. [Instance Configuration](#instance-configuration)
4. [Path Configuration](#path-configuration)
5. [Server Configuration](#server-configuration)
6. [Security Configuration](#security-configuration)
7. [TLS Configuration](#tls-configuration)
8. [Authentication Configuration](#authentication-configuration)
9. [Logging Configuration](#logging-configuration)
10. [Storage Configuration](#storage-configuration)
11. [WAL Configuration](#wal-configuration)
12. [Cache Configuration](#cache-configuration)
13. [Metrics Configuration](#metrics-configuration)
14. [Advanced Configuration](#advanced-configuration)
15. [Configuration Management](#configuration-management)
16. [Configuration Best Practices](#configuration-best-practices)

---

## Configuration Overview

### Configuration Hierarchy

RustyDB uses a layered configuration system with precedence (highest to lowest):

1. **Command-line flags** (runtime only)
2. **Environment variables** (prefix: `RUSTYDB_`)
3. **Override files** (`conf/overrides.d/*.toml`, lexicographic order)
4. **Main configuration** (`conf/rustydb.toml`)
5. **Built-in defaults**

### Configuration File Location

**Default Location**:
```
Linux:   /var/lib/rustydb/instances/{instance}/conf/rustydb.toml
Windows: C:\ProgramData\RustyDB\instances\{instance}\conf\rustydb.toml
```

**Custom Location**:
```bash
rusty-db-server --home /custom/path/to/instance
```

### Configuration Version

This documentation corresponds to **Configuration Schema v0.3.1** and **Instance Layout Spec v1.0**.

---

## Configuration File Format

RustyDB uses TOML (Tom's Obvious, Minimal Language) for configuration files.

### TOML Basics

**Comments**:
```toml
# This is a comment
key = "value"  # Inline comment
```

**Strings**:
```toml
basic_string = "Hello, World!"
literal_string = 'C:\Windows\Path'
multiline_string = """
Line 1
Line 2
"""
```

**Numbers**:
```toml
integer = 42
float = 3.14
```

**Booleans**:
```toml
enabled = true
disabled = false
```

**Arrays**:
```toml
ports = [5432, 8080, 9100]
strings = ["value1", "value2"]
```

**Tables (Sections)**:
```toml
[server]
host = "localhost"
port = 5432

[server.ipc]
enabled = true
```

### File Template

Complete configuration template is located at:
- Source: `conf/rustydb.toml`
- Installed: `{instance}/conf/rustydb.toml`

---

## Instance Configuration

Controls instance identity and metadata.

**Section**: `[instance]`

### Parameters

#### `name` (string)
- **Description**: Logical instance name
- **Default**: `"default"`
- **Recommendation**: Should match directory name when using `--instance`
- **Example**:
  ```toml
  [instance]
  name = "production"
  ```

#### `instance_id` (string, optional)
- **Description**: Stable UUID for telemetry/logging correlation
- **Default**: Auto-generated and stored in `data/meta/instance-id`
- **Format**: UUID v4
- **Example**:
  ```toml
  instance_id = "550e8400-e29b-41d4-a716-446655440000"
  ```

#### `description` (string, optional)
- **Description**: Human-readable description
- **Default**: `""`
- **Usage**: Documentation only, not used for logic
- **Example**:
  ```toml
  description = "Production database for e-commerce application"
  ```

---

## Path Configuration

Defines directory structure for instance data.

**Section**: `[paths]`

**Important**: All paths are relative to instance root (`--home`) unless absolute paths are specified.

### Parameters

#### `conf_dir` (string)
- **Description**: Configuration directory
- **Default**: `"conf"`
- **Contains**: `rustydb.toml`, `overrides.d/`, `secrets/`

#### `data_dir` (string)
- **Description**: Persistent data directory
- **Default**: `"data"`
- **Contains**: Database files, tables, indexes, WAL
- **Storage**: Requires fast SSD/NVMe, proper backups

#### `logs_dir` (string)
- **Description**: Log files directory
- **Default**: `"logs"`
- **Contains**: Application logs, audit logs, slow query logs

#### `run_dir` (string)
- **Description**: Runtime files directory
- **Default**: `"run"`
- **Contains**: PID files, Unix sockets, named pipes
- **Note**: Cleared on restart

#### `cache_dir` (string)
- **Description**: Cache directory (disposable)
- **Default**: `"cache"`
- **Contains**: Query cache, ML cache
- **Note**: Can be deleted without data loss

#### `tmp_dir` (string)
- **Description**: Temporary files directory
- **Default**: `"tmp"`
- **Contains**: Temporary sort files, intermediate results
- **Note**: Cleared periodically

#### `backup_dir` (string)
- **Description**: Backup storage directory
- **Default**: `"backup"`
- **Contains**: Database backups
- **Recommendation**: Use separate volume or network storage

#### `diag_dir` (string)
- **Description**: Diagnostics directory
- **Default**: `"diag"`
- **Contains**: Debug bundles, core dumps, diagnostic reports

### Example

```toml
[paths]
conf_dir = "conf"
data_dir = "/data/rustydb"  # Absolute path to fast storage
logs_dir = "/var/log/rustydb"
run_dir = "run"
cache_dir = "/cache/rustydb"  # Separate volume for cache
tmp_dir = "/tmp/rustydb"
backup_dir = "/backup/rustydb"  # Network storage
diag_dir = "diag"
```

---

## Server Configuration

Controls network binding and connection parameters.

**Section**: `[server]`

### Network Parameters

#### `listen_host` (string)
- **Description**: IP address to bind
- **Default**: `"127.0.0.1"` (localhost only)
- **Production**: `"0.0.0.0"` (all interfaces)
- **Security**: Use `127.0.0.1` for development, specific IP or `0.0.0.0` with firewall for production
- **Example**:
  ```toml
  listen_host = "0.0.0.0"  # Production
  listen_host = "10.0.1.100"  # Specific interface
  ```

#### `listen_port` (integer)
- **Description**: Port number for database connections
- **Default**: `54321`
- **Range**: 1024-65535
- **Note**: Standard PostgreSQL port is 5432, RustyDB uses 54321 to avoid conflicts
- **Example**:
  ```toml
  listen_port = 5432  # Use PostgreSQL standard port
  ```

#### `admin_listen_host` (string, optional)
- **Description**: Separate IP for admin/metrics endpoints
- **Default**: Same as `listen_host`
- **Use Case**: Isolate admin traffic to management network
- **Example**:
  ```toml
  admin_listen_host = "10.0.2.100"  # Management network
  ```

#### `admin_listen_port` (integer, optional)
- **Description**: Separate port for admin endpoints
- **Default**: `listen_port + 1`
- **Example**:
  ```toml
  admin_listen_port = 54322
  ```

### Connection Parameters

#### `max_connections` (integer)
- **Description**: Maximum concurrent connections
- **Default**: `500`
- **Range**: 10-10000
- **Calculation**: `max_connections = expected_concurrent_clients × 1.2 + 50`
- **Memory Impact**: Each connection uses ~1-2 MB
- **Example**:
  ```toml
  max_connections = 1000  # High-traffic production
  ```

#### `idle_timeout_ms` (integer)
- **Description**: Idle connection timeout in milliseconds
- **Default**: `300000` (5 minutes)
- **Recommendation**: 300000-1800000 (5-30 minutes)
- **Example**:
  ```toml
  idle_timeout_ms = 600000  # 10 minutes
  ```

#### `request_timeout_ms` (integer)
- **Description**: Request processing timeout in milliseconds
- **Default**: `30000` (30 seconds)
- **Use Case**: Prevent slow queries from blocking connections
- **Example**:
  ```toml
  request_timeout_ms = 60000  # 1 minute for complex queries
  ```

#### `query_timeout_ms` (integer)
- **Description**: Maximum query execution time (0 = unlimited)
- **Default**: `0`
- **Production Recommendation**: Set to prevent runaway queries
- **Example**:
  ```toml
  query_timeout_ms = 300000  # 5 minutes max
  ```

#### `backlog` (integer)
- **Description**: Connection backlog for accept queue
- **Default**: `1024`
- **Range**: 128-4096
- **Use Case**: Handle bursts of new connections
- **Example**:
  ```toml
  backlog = 2048  # High-traffic systems
  ```

### IPC Configuration

**Section**: `[server.ipc]`

#### `enabled` (boolean)
- **Description**: Enable Unix domain sockets (Linux) or named pipes (Windows)
- **Default**: `true`
- **Performance**: IPC is faster than TCP for local connections
- **Example**:
  ```toml
  [server.ipc]
  enabled = true
  ```

#### `path` (string)
- **Description**: Socket/pipe directory relative to `run_dir`
- **Default**: `"sockets"`
- **Linux**: Directory for Unix domain sockets
- **Windows**: Named pipes namespace mapping

#### `name` (string, optional)
- **Description**: Socket/pipe name
- **Default**: `"rustydb"`
- **Linux**: Creates socket at `{run_dir}/{path}/{name}.sock`
- **Windows**: Creates pipe at `\\.\pipe\{name}`

---

## Security Configuration

Controls security modes and behaviors.

**Section**: `[security]`

### Parameters

#### `mode` (string)
- **Description**: Security mode preset
- **Default**: `"dev"`
- **Options**:
  - `"dev"`: Permissive defaults, local-only, verbose logging, no TLS required
  - `"prod"`: Safer defaults, stricter validation, TLS recommended
- **Impact**:
  - **dev mode**: Disables authentication requirements, allows localhost access, detailed error messages
  - **prod mode**: Enforces authentication, requires explicit configuration, sanitized errors
- **Example**:
  ```toml
  [security]
  mode = "prod"  # Production deployment
  ```

---

## TLS Configuration

Controls Transport Layer Security for encrypted connections.

**Section**: `[tls]`

### Parameters

#### `enabled` (boolean)
- **Description**: Enable TLS/SSL encryption
- **Default**: `false`
- **Production Recommendation**: `true`
- **Example**:
  ```toml
  [tls]
  enabled = true
  ```

#### `cert_path` (string)
- **Description**: Path to server certificate
- **Default**: `"secrets/tls/server.crt"`
- **Format**: PEM-encoded X.509 certificate
- **Relative To**: `conf_dir`
- **Example**:
  ```toml
  cert_path = "secrets/tls/server.crt"
  cert_path = "/etc/ssl/certs/rustydb.crt"  # Absolute
  ```

#### `key_path` (string)
- **Description**: Path to private key
- **Default**: `"secrets/tls/server.key"`
- **Format**: PEM-encoded private key (RSA, ECDSA)
- **Security**: Must be readable only by rustydb user (mode 600)
- **Example**:
  ```toml
  key_path = "secrets/tls/server.key"
  ```

#### `ca_path` (string, optional)
- **Description**: Path to CA certificate for client verification
- **Default**: None
- **Use Case**: Mutual TLS (mTLS) client authentication
- **Example**:
  ```toml
  ca_path = "secrets/tls/ca.crt"
  ```

#### `require_client_cert` (boolean)
- **Description**: Require client certificates (mutual TLS)
- **Default**: `false`
- **Requires**: `ca_path` must be set
- **Use Case**: High-security environments, zero-trust networks
- **Example**:
  ```toml
  require_client_cert = true
  ```

#### `min_version` (string)
- **Description**: Minimum TLS version
- **Default**: `"1.2"`
- **Options**: `"1.2"`, `"1.3"`
- **Recommendation**: Use `"1.3"` for new deployments
- **Example**:
  ```toml
  min_version = "1.3"
  ```

### Example: Full TLS Configuration

```toml
[tls]
enabled = true
cert_path = "secrets/tls/server.crt"
key_path = "secrets/tls/server.key"
ca_path = "secrets/tls/ca.crt"
require_client_cert = false
min_version = "1.3"
```

---

## Authentication Configuration

Controls user authentication and session management.

**Section**: `[auth]`

### Parameters

#### `mode` (string)
- **Description**: Authentication mode
- **Default**: `"none"`
- **Options**:
  - `"none"`: No authentication (dev only, **not recommended for production**)
  - `"password"`: Username/password authentication
  - `"mtls"`: Mutual TLS client certificate authentication
  - `"token"`: Token-based authentication (JWT, API keys)
  - `"ldap"`: LDAP/Active Directory authentication
  - `"kerberos"`: Kerberos authentication
- **Example**:
  ```toml
  [auth]
  mode = "password"  # Production
  ```

#### `users_path` (string, optional)
- **Description**: Path to user database file
- **Default**: None (use internal user management)
- **Format**: JSON file with user credentials
- **Location**: Keep under `conf/secrets/auth/`
- **Security**: Must be readable only by rustydb user (mode 600)
- **Example**:
  ```toml
  users_path = "secrets/auth/users.json"
  ```

#### `session_timeout_ms` (integer)
- **Description**: Session timeout in milliseconds
- **Default**: `1800000` (30 minutes)
- **Range**: 60000-86400000 (1 minute to 24 hours)
- **Recommendation**: 1800000-3600000 (30-60 minutes)
- **Example**:
  ```toml
  session_timeout_ms = 3600000  # 1 hour
  ```

#### `max_failed_attempts` (integer)
- **Description**: Maximum failed login attempts before lockout
- **Default**: `5`
- **Range**: 3-100
- **Security**: Lower value = stricter
- **Example**:
  ```toml
  max_failed_attempts = 3  # Strict
  ```

#### `lockout_duration_ms` (integer)
- **Description**: Account lockout duration in milliseconds
- **Default**: `300000` (5 minutes)
- **Range**: 60000-3600000 (1 minute to 1 hour)
- **Example**:
  ```toml
  lockout_duration_ms = 900000  # 15 minutes
  ```

### Example: Production Authentication

```toml
[auth]
mode = "password"
session_timeout_ms = 1800000
max_failed_attempts = 5
lockout_duration_ms = 300000
```

---

## Logging Configuration

Controls application logging behavior.

**Section**: `[logging]`

### Parameters

#### `mode` (string)
- **Description**: Logging destination
- **Default**: `"file"`
- **Options**:
  - `"file"`: Write to `logs_dir` (traditional deployment)
  - `"stdout"`: Emit to stdout/stderr (container/cloud deployment)
- **Recommendation**: `"file"` for VMs, `"stdout"` for containers
- **Example**:
  ```toml
  [logging]
  mode = "stdout"  # Kubernetes deployment
  ```

#### `format` (string)
- **Description**: Log format
- **Default**: `"json"`
- **Options**:
  - `"json"`: Structured JSON (recommended for log aggregation)
  - `"text"`: Human-readable text
- **Recommendation**: `"json"` for production (Elasticsearch, Splunk, etc.)
- **Example**:
  ```toml
  format = "json"
  ```

#### `level` (string)
- **Description**: Minimum log level
- **Default**: `"info"`
- **Options**: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`
- **Performance**: Lower levels generate more logs, impact performance
- **Recommendation**:
  - Production: `"info"` or `"warn"`
  - Debugging: `"debug"` or `"trace"`
- **Example**:
  ```toml
  level = "warn"  # Production
  ```

#### `audit_enabled` (boolean)
- **Description**: Enable separate audit log channel
- **Default**: `false`
- **Use Case**: Security compliance (HIPAA, PCI-DSS, SOC 2)
- **Output**: `{logs_dir}/audit.log` (always JSON format)
- **Example**:
  ```toml
  audit_enabled = true
  ```

#### `rotate` (boolean)
- **Description**: Enable log rotation
- **Default**: `true`
- **Note**: Internal rotation; external tools (logrotate) can also be used
- **Example**:
  ```toml
  rotate = true
  ```

#### `max_files` (integer)
- **Description**: Maximum number of rotated log files to keep
- **Default**: `10`
- **Range**: 1-1000
- **Example**:
  ```toml
  max_files = 30  # Keep 30 days of logs
  ```

#### `max_file_size_mb` (integer)
- **Description**: Maximum log file size in MB before rotation
- **Default**: `100`
- **Range**: 1-1000
- **Example**:
  ```toml
  max_file_size_mb = 500
  ```

#### `include_timestamps` (boolean)
- **Description**: Include timestamps in log entries
- **Default**: `true`
- **Recommendation**: Always `true` for production

#### `include_source_location` (boolean)
- **Description**: Include source file and line number
- **Default**: `false`
- **Performance**: Slight overhead
- **Use Case**: Debugging
- **Example**:
  ```toml
  include_source_location = true  # Debug mode
  ```

### Example: Production Logging

```toml
[logging]
mode = "file"
format = "json"
level = "info"
audit_enabled = true
rotate = true
max_files = 30
max_file_size_mb = 100
include_timestamps = true
include_source_location = false
```

---

## Storage Configuration

Controls data storage and durability.

**Section**: `[storage]`

### Parameters

#### `fsync` (boolean)
- **Description**: Force fsync after writes
- **Default**: `true`
- **Durability**: `true` = durable, `false` = faster but risk data loss
- **Recommendation**: Always `true` for production
- **Example**:
  ```toml
  [storage]
  fsync = true
  ```

#### `sync_interval_ms` (integer)
- **Description**: Interval between forced syncs in milliseconds
- **Default**: `1000`
- **Range**: 100-10000
- **Tradeoff**: Lower = more durable, higher = better performance
- **Example**:
  ```toml
  sync_interval_ms = 500  # Strict durability
  ```

#### `page_size` (integer)
- **Description**: Page size in bytes
- **Default**: `4096` (4 KB)
- **Options**: Must be power of 2 (typically 4096 or 8192)
- **Warning**: **Cannot be changed** after initialization
- **Example**:
  ```toml
  page_size = 8192  # 8 KB pages (set before initialization only)
  ```

#### `buffer_pool_pages` (integer)
- **Description**: Buffer pool size in pages
- **Default**: `1000` (4 MB with 4KB pages)
- **Calculation**: `buffer_pool_pages = desired_memory_mb × 1024 / page_size_kb`
- **Example**: For 1 GB buffer pool with 4KB pages: `1024 × 1024 / 4 = 262144`
- **Recommendation**:
  - Development: 1000 (4 MB)
  - Small production: 65536 (256 MB)
  - Medium production: 262144 (1 GB)
  - Large production: 1048576 (4 GB)
- **Example**:
  ```toml
  buffer_pool_pages = 262144  # 1 GB
  ```

#### `compression` (string, optional)
- **Description**: Compression algorithm for data pages
- **Default**: None (no compression)
- **Options**: `"lz4"`, `"zstd"`, `"snappy"`
- **Tradeoff**: CPU usage vs storage savings
- **Recommendation**: `"lz4"` for good balance
- **Example**:
  ```toml
  compression = "lz4"
  ```

#### `encryption_enabled` (boolean, optional)
- **Description**: Enable encryption at rest
- **Default**: `false`
- **Requires**: Encryption key configuration
- **Performance**: ~10-15% overhead
- **Example**:
  ```toml
  encryption_enabled = true
  ```

### Example: Production Storage

```toml
[storage]
fsync = true
sync_interval_ms = 1000
page_size = 4096
buffer_pool_pages = 262144  # 1 GB
compression = "lz4"
encryption_enabled = true
```

---

## WAL Configuration

Controls Write-Ahead Log for durability and replication.

**Section**: `[wal]`

### Parameters

#### `enabled` (boolean)
- **Description**: Enable Write-Ahead Logging
- **Default**: `true`
- **Recommendation**: Always `true` for production (required for crash recovery)
- **Example**:
  ```toml
  [wal]
  enabled = true
  ```

#### `dir` (string)
- **Description**: WAL directory relative to `data_dir`
- **Default**: `"wal"`
- **Recommendation**: Use separate fast storage for optimal performance
- **Example**:
  ```toml
  dir = "wal"
  dir = "/fast-storage/wal"  # Absolute path to dedicated SSD
  ```

#### `max_segment_mb` (integer)
- **Description**: Maximum WAL segment size in MB
- **Default**: `64`
- **Range**: 16-256
- **Impact**: Larger segments = fewer files, but longer recovery time
- **Example**:
  ```toml
  max_segment_mb = 128
  ```

#### `checkpoint_interval_ms` (integer)
- **Description**: Checkpoint interval in milliseconds
- **Default**: `60000` (1 minute)
- **Range**: 10000-600000 (10 seconds to 10 minutes)
- **Tradeoff**: Shorter = less recovery time, more I/O overhead
- **Example**:
  ```toml
  checkpoint_interval_ms = 300000  # 5 minutes
  ```

#### `sync_mode` (string)
- **Description**: Synchronous commit mode
- **Default**: `"local"`
- **Options**:
  - `"off"`: Async (fastest, risk data loss)
  - `"local"`: Sync to local disk (standard)
  - `"remote_write"`: Wait for standby to receive (replication)
  - `"remote_apply"`: Wait for standby to apply (strongest)
- **Recommendation**: `"local"` for single-node, `"remote_write"` for HA
- **Example**:
  ```toml
  sync_mode = "remote_write"  # High availability
  ```

#### `archive_enabled` (boolean)
- **Description**: Enable WAL archiving for point-in-time recovery
- **Default**: `false`
- **Use Case**: PITR, disaster recovery
- **Example**:
  ```toml
  archive_enabled = true
  ```

#### `archive_command` (string, optional)
- **Description**: Command to archive WAL segments
- **Placeholders**: `%f` = filename, `%p` = path
- **Example**:
  ```toml
  archive_command = "cp %p /archive/wal/%f"
  archive_command = "aws s3 cp %p s3://backup-bucket/wal/%f"
  ```

### Example: HA Configuration

```toml
[wal]
enabled = true
dir = "/fast-storage/wal"
max_segment_mb = 64
checkpoint_interval_ms = 60000
sync_mode = "remote_write"
archive_enabled = true
archive_command = "aws s3 cp %p s3://rustydb-backup/wal/%f"
```

---

## Cache Configuration

Controls caching behavior.

**Section**: `[cache]`

### Parameters

#### `enabled` (boolean)
- **Description**: Enable cache system
- **Default**: `true`
- **Note**: Cache is disposable (can be cleared without data loss)

#### `max_size_mb` (integer)
- **Description**: Maximum cache size in MB
- **Default**: `512`
- **Range**: 100-10000
- **Recommendation**: 1-5% of available memory

#### `ml_enabled` (boolean)
- **Description**: Enable ML model caching
- **Default**: `true`
- **Use Case**: In-database ML operations

#### `query_cache_enabled` (boolean)
- **Description**: Enable query result caching
- **Default**: `true`
- **Performance**: Significant speedup for repeated queries
- **Example**:
  ```toml
  query_cache_enabled = true
  ```

#### `query_cache_ttl_ms` (integer)
- **Description**: Query cache TTL in milliseconds
- **Default**: `60000` (1 minute)
- **Range**: 1000-3600000
- **Example**:
  ```toml
  query_cache_ttl_ms = 300000  # 5 minutes
  ```

#### `query_cache_max_entries` (integer)
- **Description**: Maximum entries in query cache
- **Default**: `10000`
- **Range**: 100-1000000
- **Example**:
  ```toml
  query_cache_max_entries = 50000
  ```

---

## Metrics Configuration

Controls Prometheus-compatible metrics exposure.

**Section**: `[metrics]`

### Parameters

#### `enabled` (boolean)
- **Description**: Enable metrics collection
- **Default**: `true`

#### `mode` (string)
- **Description**: Metrics exposure mode
- **Default**: `"pull"`
- **Options**:
  - `"pull"`: Prometheus scrape endpoint
  - `"push"`: Push to collector

#### `listen_host` (string)
- **Description**: Metrics endpoint bind address
- **Default**: `"127.0.0.1"`

#### `listen_port` (integer)
- **Description**: Metrics endpoint port
- **Default**: `9100`

#### `path` (string)
- **Description**: Metrics endpoint path (pull mode)
- **Default**: `"/metrics"`

### Example

```toml
[metrics]
enabled = true
mode = "pull"
listen_host = "0.0.0.0"
listen_port = 9100
path = "/metrics"
```

---

## Advanced Configuration

### Clustering (Optional)

**Section**: `[clustering]`

```toml
[clustering]
enabled = false
node_id = "node1"
cluster_name = "rustydb-cluster"
seeds = ["10.0.1.101:7000", "10.0.1.102:7000", "10.0.1.103:7000"]
```

### Replication (Optional)

**Section**: `[replication]`

```toml
[replication]
role = "primary"  # "primary", "replica", "standby"
primary_host = "10.0.1.101"
primary_port = 5432
```

### Resource Limits (Optional)

**Section**: `[resources]`

```toml
[resources]
max_memory_mb = 8192  # 0 = unlimited
max_cpu_percent = 80  # 0 = unlimited
max_io_bandwidth_mbps = 1000  # 0 = unlimited
```

---

## Configuration Management

### Reload Configuration

**Without Restart** (some settings):
```bash
# REST API
curl -X POST http://localhost:8080/api/v1/admin/config/reload

# CLI
rusty-db-cli --command "SELECT pg_reload_conf();"
```

**With Restart** (most settings):
```bash
# Linux
sudo systemctl restart rustydb

# Windows
sc stop RustyDB_default && sc start RustyDB_default
```

### Validate Configuration

```bash
rusty-db-server --check-config /path/to/rustydb.toml
```

### Backup Configuration

```bash
# Linux
sudo cp /var/lib/rustydb/instances/default/conf/rustydb.toml \
       /var/lib/rustydb/instances/default/conf/rustydb.toml.backup

# Add to version control
git add conf/rustydb.toml
git commit -m "Update production configuration"
```

---

## Configuration Best Practices

### 1. Use Configuration Management

- Store in version control (Git)
- Use infrastructure-as-code (Terraform, Ansible)
- Document all changes

### 2. Separate Environments

```
conf/
├── rustydb.toml          # Base configuration
└── overrides.d/
    ├── 10-dev.toml       # Development overrides
    ├── 20-staging.toml   # Staging overrides
    └── 30-prod.toml      # Production overrides
```

### 3. Secure Sensitive Data

- Keep secrets in `conf/secrets/` (mode 600)
- Use environment variables for passwords
- Consider HashiCorp Vault or similar

### 4. Document Changes

```toml
# Changed 2025-12-28: Increased buffer pool for better performance
# Previous value: 65536 (256 MB)
# New value: 262144 (1 GB)
# Reason: Performance improvement for analytical queries
buffer_pool_pages = 262144
```

### 5. Test Configuration Changes

1. Test in development environment
2. Validate with `--check-config`
3. Deploy to staging
4. Monitor metrics after change
5. Document results
6. Deploy to production

### 6. Performance Tuning Workflow

1. Establish baseline metrics
2. Change one parameter at a time
3. Monitor for 24-48 hours
4. Measure impact
5. Keep or revert
6. Document findings

---

## Related Documentation

- [OPERATIONS_OVERVIEW.md](./OPERATIONS_OVERVIEW.md) - Operations architecture
- [INSTALLATION.md](./INSTALLATION.md) - Installation procedures
- [MONITORING.md](./MONITORING.md) - Monitoring setup
- [MAINTENANCE.md](./MAINTENANCE.md) - Maintenance procedures

---

**Document Maintained By**: Enterprise Documentation Agent 4
**RustyDB Version**: 0.6.0
**Configuration Schema**: v0.3.1
