# RustyDB Database Administration Guide
## Version 0.5.1 - Enterprise Edition

**Document Version:** 1.0
**Last Updated:** December 2024
**Target Audience:** Database Administrators, System Administrators
**Classification:** Enterprise Production Administration

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Server Management](#2-server-management)
3. [Configuration Management](#3-configuration-management)
4. [Storage Administration](#4-storage-administration)
5. [Memory Administration](#5-memory-administration)
6. [User and Schema Management](#6-user-and-schema-management)
7. [Space Management](#7-space-management)
8. [Routine Maintenance](#8-routine-maintenance)
9. [Job Scheduling](#9-job-scheduling)
10. [Performance Management](#10-performance-management)
11. [Backup and Recovery](#11-backup-and-recovery)
12. [Security Administration](#12-security-administration)
13. [Appendices](#13-appendices)

---

## 1. Introduction

### 1.1 About This Guide

This administration guide provides comprehensive procedures for managing RustyDB v0.5.1 in enterprise environments. It follows Oracle DBA best practices adapted for RustyDB's architecture.

**Prerequisites:**
- RustyDB v0.5.1 installed (see DEPLOYMENT_GUIDE.md)
- Basic understanding of database concepts
- Root or rustydb user access
- Familiarity with Linux command line

**Related Documentation:**
- **OPERATIONS.md** - Advanced operations, monitoring, and workload intelligence
- **DEPLOYMENT_GUIDE.md** - Installation and deployment procedures
- **SECURITY_ARCHITECTURE.md** - Comprehensive security guide

### 1.2 DBA Responsibilities

As a RustyDB DBA, your key responsibilities include:

1. **Availability** - Ensuring 24/7 database uptime
2. **Performance** - Monitoring and optimizing database performance
3. **Security** - Protecting data from unauthorized access
4. **Capacity Planning** - Managing storage and resource growth
5. **Backup and Recovery** - Protecting against data loss
6. **User Management** - Creating and managing database users
7. **Monitoring** - Proactive problem detection and resolution
8. **Maintenance** - Routine database maintenance tasks

### 1.3 Instance Architecture

A RustyDB instance consists of:

```
┌─────────────────────────────────────────────────────────────┐
│                    RustyDB Instance                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Memory     │  │   Processes  │  │   Storage    │      │
│  │  Structures  │  │              │  │              │      │
│  ├──────────────┤  ├──────────────┤  ├──────────────┤      │
│  │ Buffer Pool  │  │ Server Proc  │  │ Data Files   │      │
│  │ Shared Mem   │  │ Background   │  │ WAL Files    │      │
│  │ WAL Buffer   │  │ Worker Pool  │  │ Temp Files   │      │
│  │ Query Cache  │  │ Checkpointer │  │ Archive Logs │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Key Components:**

| Component | Description | Location |
|-----------|-------------|----------|
| Data Files | Permanent database storage | /var/lib/rustydb/data |
| WAL Files | Write-Ahead Log for durability | /var/lib/rustydb/wal |
| Control Files | Instance metadata | /var/lib/rustydb/data/meta |
| Archive Logs | Historical WAL segments | /var/lib/rustydb/backup/archive |
| Configuration | rustydb.toml | /etc/rustydb/config.toml |
| Log Files | Application and audit logs | /var/log/rustydb |

---

## 2. Server Management

### 2.1 Starting the Database

#### Standard Startup

```bash
# Start database using systemd (recommended)
sudo systemctl start rustydb

# Verify startup
sudo systemctl status rustydb

# Check logs for successful startup
sudo journalctl -u rustydb -f
```

Expected log output:
```
[INFO] RustyDB v0.5.1 starting
[INFO] Loading configuration from /etc/rustydb/config.toml
[INFO] Buffer pool initialized: ~8 MB (1000 pages × 8192 bytes)
[INFO] WAL system initialized
[INFO] Transaction manager started
[INFO] Network listener started on 0.0.0.0:5432
[INFO] REST API server started on 0.0.0.0:8080
[INFO] RustyDB ready for connections
```

#### Manual Startup

```bash
# Start manually (for debugging)
sudo -u rustydb rusty-db-server --config /etc/rustydb/config.toml

# Start in foreground with verbose logging
sudo -u rustydb rusty-db-server --config /etc/rustydb/config.toml --log-level debug
```

#### Automatic Startup on Boot

```bash
# Enable automatic startup
sudo systemctl enable rustydb

# Verify auto-start is enabled
sudo systemctl is-enabled rustydb
# Expected output: enabled
```

### 2.2 Startup Modes

RustyDB supports different startup modes for various scenarios:

#### NORMAL Mode (Default)

Starts the database with full functionality.

```bash
# Normal startup (default)
sudo systemctl start rustydb
```

**Use Cases:**
- Regular production operations
- All services available
- Full read/write access

#### MOUNT Mode

Starts the instance but does not open the database. Used for maintenance operations.

```bash
# Start in mount mode
sudo -u rustydb rusty-db-server --config /etc/rustydb/config.toml --mount-only

# Or via API
curl -X POST http://localhost:8080/api/v1/admin/startup \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -d '{"mode":"MOUNT"}'
```

**Use Cases:**
- Database recovery operations
- Tablespace maintenance
- Backup operations
- Database upgrades

**Available Operations in MOUNT Mode:**
- Read control files
- Perform recovery
- Rename data files
- Enable/disable archive logging

#### RESTRICT Mode

Opens the database but only allows connections from users with RESTRICTED SESSION privilege.

```bash
# Start in restrict mode
sudo -u rustydb rusty-db-server --config /etc/rustydb/config.toml --restrict

# Or alter running instance
rusty-db-cli << EOF
ALTER SYSTEM ENABLE RESTRICTED SESSION;
EOF
```

**Use Cases:**
- Maintenance operations
- Data loading
- Emergency patches
- User management

**Granting RESTRICT privilege:**
```sql
-- Grant restricted session privilege
GRANT RESTRICTED SESSION TO admin_user;
```

### 2.3 Stopping the Database

#### Graceful Shutdown (NORMAL)

Waits for all active sessions to complete before shutting down.

```bash
# Graceful shutdown
sudo systemctl stop rustydb

# Or via API
curl -X POST http://localhost:8080/api/v1/admin/shutdown \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -d '{"mode":"NORMAL"}'
```

**Characteristics:**
- Waits for active transactions to complete
- Allows users to disconnect gracefully
- Performs checkpoint before shutdown
- Can take several minutes
- Safest shutdown method

#### Immediate Shutdown (IMMEDIATE)

Disconnects all users and performs rollback before shutting down.

```bash
# Immediate shutdown via API
curl -X POST http://localhost:8080/api/v1/admin/shutdown \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -d '{"mode":"IMMEDIATE"}'

# Or using CLI
rusty-db-cli << EOF
SHUTDOWN IMMEDIATE;
EOF
```

**Characteristics:**
- Terminates active transactions (rollback)
- Disconnects all users immediately
- Performs checkpoint
- Faster than NORMAL
- Safe for production

#### Emergency Shutdown (ABORT)

Forces immediate shutdown without cleanup (last resort only).

```bash
# Emergency shutdown (use with caution)
sudo systemctl kill -s SIGKILL rustydb

# Or via API
curl -X POST http://localhost:8080/api/v1/admin/shutdown \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -d '{"mode":"ABORT"}'
```

**WARNING:** Only use ABORT shutdown in emergencies!

**Characteristics:**
- No checkpoint performed
- No transaction rollback
- Immediate termination
- Requires crash recovery on restart
- Use only when other methods fail

### 2.4 Process Architecture

RustyDB uses a multi-threaded async architecture:

#### Server Process

The main server process handles:
- Connection management
- Query routing
- Transaction coordination
- Background maintenance

```bash
# View main server process
ps aux | grep rusty-db-server

# Expected output:
# rustydb  1234  5.0 2.3 8388608 3145728 ?  Ssl  10:00  0:30 /usr/local/bin/rusty-db-server
```

#### Background Workers

```bash
# View all RustyDB processes
pstree -p $(pgrep -f rusty-db-server)

# Example output:
# rusty-db-server(1234)───┬─{checkpointer}(1235)
#                         ├─{wal_writer}(1236)
#                         ├─{stats_collector}(1237)
#                         ├─{auto_vacuum}(1238)
#                         └─{worker_pool}(1239)
```

**Background Process Roles:**

| Process | Purpose | Configuration |
|---------|---------|---------------|
| Checkpointer | Writes dirty buffers to disk | checkpoint_interval_sec |
| WAL Writer | Writes WAL records to disk | wal.sync_mode |
| Stats Collector | Collects performance statistics | stats_enabled |
| Auto Vacuum | Automatic space reclamation | auto_vacuum_enabled |
| Worker Pool | Parallel query execution | max_parallel_workers |

#### Monitoring Active Connections

```sql
-- View all active connections
SELECT
    session_id,
    user_name,
    client_address,
    state,
    current_query,
    connected_at
FROM v$session
WHERE state = 'ACTIVE';

-- Count connections by state
SELECT
    state,
    count(*) as session_count
FROM v$session
GROUP BY state;
```

#### Kill Specific Session

```sql
-- Kill a problematic session
ALTER SYSTEM KILL SESSION 'session_id';

-- Or via API
curl -X DELETE http://localhost:8080/api/v1/connections/session_12345 \
  -H "Authorization: Bearer ${ADMIN_TOKEN}"
```

### 2.5 Instance Health Checks

#### Quick Health Check

```bash
#!/bin/bash
# /usr/local/bin/rustydb-healthcheck.sh

echo "=== RustyDB Health Check ==="
echo "Date: $(date)"

# Check if server is running
if systemctl is-active --quiet rustydb; then
    echo "✓ Server Status: RUNNING"
else
    echo "✗ Server Status: STOPPED"
    exit 1
fi

# Check connectivity
if rusty-db-cli --command "SELECT 1" > /dev/null 2>&1; then
    echo "✓ Database Connectivity: OK"
else
    echo "✗ Database Connectivity: FAILED"
    exit 1
fi

# Check API health
if curl -s http://localhost:8080/api/v1/admin/health | grep -q '"status":"HEALTHY"'; then
    echo "✓ API Health: OK"
else
    echo "✗ API Health: FAILED"
    exit 1
fi

# Check disk space
DISK_USAGE=$(df -h /var/lib/rustydb | awk 'NR==2 {print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -lt 90 ]; then
    echo "✓ Disk Space: ${DISK_USAGE}% used"
else
    echo "⚠ Disk Space: ${DISK_USAGE}% used (WARNING)"
fi

# Check memory usage
MEMORY_USAGE=$(curl -s http://localhost:8080/api/v1/metrics | jq -r '.data.memory.buffer_pool_usage_percent')
echo "✓ Buffer Pool Usage: ${MEMORY_USAGE}%"

# Check active connections
CONNECTIONS=$(curl -s http://localhost:8080/api/v1/connections | jq -r '.data | length')
echo "✓ Active Connections: ${CONNECTIONS}"

echo "=== Health Check Complete ==="
```

#### Comprehensive Health Check

```sql
-- Run comprehensive health check query
SELECT * FROM v$health_check;

-- Expected columns:
-- component          VARCHAR(100)
-- status             VARCHAR(20)    -- HEALTHY, DEGRADED, UNHEALTHY, CRITICAL
-- message            TEXT
-- last_check_time    TIMESTAMP
-- details            JSONB
```

Example output:
```
component             | status   | message
----------------------+----------+--------------------------------
database              | HEALTHY  | All systems operational
buffer_pool           | HEALTHY  | Hit ratio: 98.5%
wal_system            | HEALTHY  | WAL archiving up to date
replication           | HEALTHY  | Replication lag: 0.5s
connections           | HEALTHY  | 45/500 connections in use
storage               | HEALTHY  | 65% disk space used
memory                | HEALTHY  | 70% buffer pool used
```

---

## 3. Configuration Management

### 3.1 Configuration File Structure

RustyDB uses TOML format for configuration. The primary configuration file is located at `/etc/rustydb/config.toml`.

**Configuration Hierarchy:**

```
/etc/rustydb/
├── config.toml           # Main configuration file
├── overrides.d/          # Override configurations (optional)
│   ├── 01-network.toml
│   ├── 02-security.toml
│   └── 03-performance.toml
└── secrets/              # Sensitive configuration
    ├── tls/
    │   ├── server.crt
    │   ├── server.key
    │   └── ca.crt
    └── auth/
        └── users.json
```

### 3.2 Parameter Categories

RustyDB configuration parameters are organized into categories:

#### Instance Parameters

```toml
[instance]
name = "production"                  # Instance name
description = "Production Database"  # Human-readable description
```

#### Server Parameters

```toml
[server]
listen_host = "0.0.0.0"             # Listening address
listen_port = 5432                   # Listening port
max_connections = 1000               # Maximum concurrent connections
idle_timeout_ms = 300000             # Idle connection timeout (5 min)
request_timeout_ms = 30000           # Request timeout (30 sec)
query_timeout_ms = 600000            # Query timeout (10 min)
backlog = 1024                       # Connection backlog
```

#### Storage Parameters

```toml
[storage]
data_dir = "/var/lib/rustydb/data"  # Data directory
fsync = true                         # Enable fsync for durability
sync_interval_ms = 1000              # Sync interval
page_size = 8192                     # Page size in bytes (8 KB)
buffer_pool_pages = 1000             # Buffer pool size (~8 MB with 8KB pages)
```

#### Memory Parameters

```toml
[memory]
buffer_pool_size = 8192000           # ~8 MB (1000 pages × 8192 bytes)
shared_memory_size = 2147483648      # 2 GB
wal_buffer_size = 67108864           # 64 MB
work_mem = 67108864                  # 64 MB per operation
maintenance_work_mem = 2147483648    # 2 GB for maintenance
```

#### WAL (Write-Ahead Log) Parameters

```toml
[wal]
enabled = true
dir = "wal"                          # WAL directory relative to data_dir
max_segment_mb = 64                  # Maximum WAL segment size
checkpoint_interval_ms = 300000      # Checkpoint every 5 minutes
sync_mode = "local"                  # Synchronous commit mode
archive_enabled = true               # Enable WAL archiving
archive_command = "cp %p /var/lib/rustydb/backup/archive/%f"
```

#### Performance Parameters

```toml
[performance]
parallel_query_enabled = true
max_parallel_workers = 32            # Number of parallel workers
max_parallel_workers_per_query = 8   # Workers per query
simd_enabled = true                  # Enable SIMD optimizations
io_uring_enabled = true              # Enable io_uring (Linux)
work_stealing_enabled = true         # Work-stealing scheduler
```

#### Security Parameters

```toml
[security]
mode = "prod"                        # "dev" or "prod"
tde_enabled = true                   # Transparent Data Encryption
tde_master_key_file = "/etc/rustydb/secrets/master.key"
data_masking_enabled = true
audit_log_enabled = true
audit_log_file = "/var/log/rustydb/security.log"
```

#### Logging Parameters

```toml
[logging]
mode = "file"                        # "file" or "stdout"
format = "json"                      # "json" or "text"
level = "info"                       # "trace", "debug", "info", "warn", "error"
rotate = true
max_files = 10
max_file_size_mb = 100
audit_enabled = true
```

### 3.3 Viewing Configuration

#### View Current Configuration

```sql
-- View all configuration parameters
SELECT * FROM v$parameter ORDER BY name;

-- View specific parameter
SELECT name, value, unit, description
FROM v$parameter
WHERE name = 'max_connections';

-- View parameters by category
SELECT name, value, category, is_dynamic
FROM v$parameter
WHERE category = 'memory'
ORDER BY name;
```

#### Via API

```bash
# Get all parameters
curl http://localhost:8080/api/v1/admin/config

# Get specific parameter
curl http://localhost:8080/api/v1/admin/config/max_connections
```

#### Via CLI

```bash
# Show configuration file location
rusty-db-cli << EOF
SHOW config_file;
EOF

# Show specific parameter
rusty-db-cli << EOF
SHOW max_connections;
EOF

# Show all parameters matching pattern
rusty-db-cli << EOF
SHOW PARAMETERS LIKE '%memory%';
EOF
```

### 3.4 Modifying Configuration

#### Static Parameters (Requires Restart)

Static parameters require a database restart to take effect.

```bash
# 1. Stop the database
sudo systemctl stop rustydb

# 2. Edit configuration file
sudo vim /etc/rustydb/config.toml

# Modify parameter (example: change buffer pool size)
[memory]
buffer_pool_size = 17179869184  # Change from 8 GB to 16 GB

# 3. Validate configuration
sudo -u rustydb rusty-db-server --config /etc/rustydb/config.toml --validate

# 4. Restart the database
sudo systemctl start rustydb

# 5. Verify change
rusty-db-cli << EOF
SHOW buffer_pool_size;
EOF
```

**Static Parameters:**
- buffer_pool_size
- page_size
- data_dir
- wal.dir
- listen_port
- tls settings

#### Dynamic Parameters (No Restart Required)

Dynamic parameters can be changed while the database is running.

```sql
-- Change session-level parameter
ALTER SESSION SET query_timeout_ms = 300000;

-- Change system-level parameter
ALTER SYSTEM SET max_connections = 2000;

-- Change with SCOPE clause
ALTER SYSTEM SET checkpoint_interval_ms = 600000 SCOPE = MEMORY;
-- SCOPE options:
--   MEMORY: Change takes effect immediately, lost on restart
--   SPFILE: Change saved to config file, takes effect on restart
--   BOTH: Change immediately and save to config (default)
```

**Dynamic Parameters:**
- max_connections
- query_timeout_ms
- checkpoint_interval_ms
- work_mem
- maintenance_work_mem
- log_level
- statistics parameters

#### Via API

```bash
# Update dynamic parameter
curl -X PUT http://localhost:8080/api/v1/admin/config/max_connections \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -d '{"value": 2000, "scope": "BOTH"}'
```

### 3.5 Configuration Best Practices

#### 1. Buffer Pool Sizing

**Rule of Thumb:** Allocate 25-40% of system RAM to buffer pool.

```toml
# For a 64 GB system:
[memory]
buffer_pool_size = 17179869184  # 16 GB (25%)
# or
buffer_pool_size = 25769803776  # 24 GB (37.5%)
```

#### 2. Connection Limits

**Rule of Thumb:** Set max_connections based on expected workload.

```toml
[server]
# Small to medium workload (OLTP)
max_connections = 500

# Large workload (mixed OLTP/OLAP)
max_connections = 1000

# Very large workload (web scale)
max_connections = 5000
```

**Note:** Each connection consumes memory. Monitor actual connection usage.

#### 3. WAL Configuration

**Production Settings:**

```toml
[wal]
enabled = true
max_segment_mb = 64
checkpoint_interval_ms = 300000  # 5 minutes
sync_mode = "local"              # or "remote_write" for HA
archive_enabled = true
```

#### 4. Parallel Query Configuration

**CPU-bound workloads:**

```toml
[performance]
max_parallel_workers = 32        # Set to CPU core count
max_parallel_workers_per_query = 8
```

#### 5. Maintenance Windows

**Configure maintenance schedules:**

```toml
[maintenance]
auto_vacuum_enabled = true
vacuum_schedule = "0 3 * * 0"    # 3 AM every Sunday
auto_analyze_enabled = true
checkpoint_interval_sec = 300    # 5 minutes
```

### 3.6 Configuration Validation

#### Pre-Deployment Validation

```bash
# Validate configuration file syntax
sudo -u rustydb rusty-db-server --config /etc/rustydb/config.toml --validate

# Expected output:
# [INFO] Configuration validation successful
# [INFO] All parameters within valid ranges
# [INFO] No conflicts detected
```

#### Check Configuration After Changes

```bash
# Compare current running config with file
diff <(rusty-db-cli -c "SHOW ALL" | sort) \
     <(grep -v '^#' /etc/rustydb/config.toml | sort)
```

#### Configuration Audit

```sql
-- View all non-default parameters
SELECT name, value, default_value
FROM v$parameter
WHERE value != default_value
ORDER BY name;

-- View recently changed parameters
SELECT name, old_value, new_value, changed_by, changed_at
FROM v$parameter_history
WHERE changed_at > NOW() - INTERVAL '7 days'
ORDER BY changed_at DESC;
```

---

## 4. Storage Administration

### 4.1 Storage Architecture

RustyDB uses a page-based storage architecture:

```
Data Directory Structure:
/var/lib/rustydb/data/
├── meta/                    # Instance metadata
│   ├── instance-id
│   ├── control.dat
│   └── catalog.dat
├── tablespaces/             # Tablespace data
│   ├── system/              # System tablespace
│   │   ├── data_001.dbf
│   │   ├── data_002.dbf
│   │   └── data_003.dbf
│   ├── users/               # User tablespace
│   │   └── users_001.dbf
│   └── temp/                # Temporary tablespace
│       └── temp_001.dbf
├── indexes/                 # Index files
└── lobs/                    # Large objects (LOBs)
```

### 4.2 Tablespace Management

Tablespaces are logical storage containers for database objects.

#### View Tablespaces

```sql
-- List all tablespaces
SELECT
    tablespace_name,
    status,
    total_size_mb,
    used_size_mb,
    free_size_mb,
    ROUND(used_size_mb * 100.0 / total_size_mb, 2) as usage_percent
FROM v$tablespace
ORDER BY tablespace_name;
```

Example output:
```
tablespace_name | status | total_mb | used_mb | free_mb | usage_percent
----------------+--------+----------+---------+---------+--------------
SYSTEM          | ONLINE | 1024     | 768     | 256     | 75.00
USERS           | ONLINE | 10240    | 6553    | 3687    | 64.00
TEMP            | ONLINE | 2048     | 512     | 1536    | 25.00
INDEXES         | ONLINE | 5120     | 3840    | 1280    | 75.00
```

#### Create Tablespace

```sql
-- Create permanent tablespace
CREATE TABLESPACE app_data
    DATAFILE '/var/lib/rustydb/data/tablespaces/app_data/data_001.dbf'
    SIZE 1G
    AUTOEXTEND ON
    NEXT 256M
    MAXSIZE 10G;

-- Create temporary tablespace
CREATE TEMPORARY TABLESPACE app_temp
    TEMPFILE '/var/lib/rustydb/data/tablespaces/app_temp/temp_001.dbf'
    SIZE 512M
    AUTOEXTEND ON
    NEXT 128M
    MAXSIZE 2G;
```

#### Alter Tablespace

```sql
-- Add datafile to tablespace
ALTER TABLESPACE app_data
    ADD DATAFILE '/var/lib/rustydb/data/tablespaces/app_data/data_002.dbf'
    SIZE 1G
    AUTOEXTEND ON;

-- Resize datafile
ALTER DATABASE
    DATAFILE '/var/lib/rustydb/data/tablespaces/app_data/data_001.dbf'
    RESIZE 2G;

-- Take tablespace offline
ALTER TABLESPACE app_data OFFLINE;

-- Bring tablespace online
ALTER TABLESPACE app_data ONLINE;

-- Set tablespace to read-only
ALTER TABLESPACE app_data READ ONLY;

-- Set tablespace to read-write
ALTER TABLESPACE app_data READ WRITE;
```

#### Drop Tablespace

```sql
-- Drop empty tablespace
DROP TABLESPACE app_data;

-- Drop tablespace and contents
DROP TABLESPACE app_data INCLUDING CONTENTS;

-- Drop tablespace, contents, and delete datafiles
DROP TABLESPACE app_data INCLUDING CONTENTS AND DATAFILES;
```

#### Tablespace Best Practices

1. **Separate System and User Data**
   ```sql
   -- Create separate tablespaces for different applications
   CREATE TABLESPACE app1_data DATAFILE ... SIZE 5G;
   CREATE TABLESPACE app2_data DATAFILE ... SIZE 10G;
   ```

2. **Use Autoextend Wisely**
   ```sql
   -- Enable autoextend with reasonable limits
   CREATE TABLESPACE app_data
       DATAFILE '/path/to/data.dbf' SIZE 1G
       AUTOEXTEND ON NEXT 256M MAXSIZE 50G;
   ```

3. **Monitor Tablespace Usage**
   ```bash
   # Create monitoring script
   #!/bin/bash
   rusty-db-cli << EOF
   SELECT tablespace_name, usage_percent
   FROM v$tablespace
   WHERE usage_percent > 80;
   EOF
   ```

4. **Use Multiple Datafiles**
   ```sql
   -- Spread I/O across multiple disks
   ALTER TABLESPACE large_data
       ADD DATAFILE '/disk1/data_001.dbf' SIZE 10G,
                    '/disk2/data_002.dbf' SIZE 10G,
                    '/disk3/data_003.dbf' SIZE 10G;
   ```

### 4.3 Datafile Management

#### View Datafiles

```sql
-- List all datafiles
SELECT
    file_id,
    tablespace_name,
    file_name,
    size_mb,
    status,
    autoextend,
    maxsize_mb
FROM v$datafile
ORDER BY tablespace_name, file_id;
```

#### Add Datafile

```sql
-- Add datafile to existing tablespace
ALTER TABLESPACE users
    ADD DATAFILE '/var/lib/rustydb/data/tablespaces/users/users_002.dbf'
    SIZE 2G
    AUTOEXTEND ON
    NEXT 256M
    MAXSIZE UNLIMITED;
```

#### Move/Rename Datafile

```sql
-- Step 1: Take tablespace offline
ALTER TABLESPACE users OFFLINE;

-- Step 2: Move file at OS level
-- (In another terminal)
-- sudo mv /old/path/users_001.dbf /new/path/users_001.dbf

-- Step 3: Update database
ALTER DATABASE
    RENAME FILE '/old/path/users_001.dbf'
    TO '/new/path/users_001.dbf';

-- Step 4: Bring tablespace online
ALTER TABLESPACE users ONLINE;
```

#### Check Datafile I/O Statistics

```sql
-- View datafile I/O statistics
SELECT
    file_name,
    reads,
    writes,
    read_time_ms,
    write_time_ms,
    ROUND(read_time_ms / NULLIF(reads, 0), 2) as avg_read_ms,
    ROUND(write_time_ms / NULLIF(writes, 0), 2) as avg_write_ms
FROM v$datafile_io_stats
ORDER BY reads + writes DESC
LIMIT 20;
```

### 4.4 Redo Log Management

Redo logs (WAL files) record all changes to the database for recovery purposes.

#### View Redo Log Status

```sql
-- View redo log groups
SELECT
    group_id,
    thread_id,
    sequence_number,
    size_mb,
    status,
    archived
FROM v$log
ORDER BY group_id;

-- View redo log members
SELECT
    group_id,
    member_path,
    status
FROM v$logfile
ORDER BY group_id;
```

#### Monitor WAL Generation Rate

```sql
-- WAL generation statistics
SELECT
    ROUND(wal_bytes_per_second / 1024 / 1024, 2) as wal_mb_per_sec,
    ROUND(wal_records_per_second, 2) as wal_records_per_sec,
    last_checkpoint_time,
    EXTRACT(EPOCH FROM (NOW() - last_checkpoint_time)) as seconds_since_checkpoint
FROM v$wal_stats;
```

#### Check WAL Archiving

```sql
-- View archive log status
SELECT
    sequence,
    name,
    first_time,
    completion_time,
    archived,
    applied,
    deleted
FROM v$archived_log
ORDER BY sequence DESC
LIMIT 20;
```

### 4.5 Archive Log Management

Archive logs are historical WAL segments required for backup and recovery.

#### Enable Archive Logging

```sql
-- Check if archive logging is enabled
SELECT log_mode FROM v$database;
-- Returns: NOARCHIVELOG or ARCHIVELOG

-- Enable archive logging (requires restart)
SHUTDOWN IMMEDIATE;
STARTUP MOUNT;
ALTER DATABASE ARCHIVELOG;
ALTER DATABASE OPEN;
```

#### Configure Archive Destination

```toml
# /etc/rustydb/config.toml
[wal]
archive_enabled = true
archive_dir = "/var/lib/rustydb/backup/archive"
archive_command = "cp %p /var/lib/rustydb/backup/archive/%f"
```

#### View Archive Log Disk Usage

```bash
# Check archive log directory size
du -sh /var/lib/rustydb/backup/archive

# Count archive log files
ls /var/lib/rustydb/backup/archive | wc -l

# View oldest and newest archive logs
ls -lt /var/lib/rustydb/backup/archive | tail -1  # Oldest
ls -lt /var/lib/rustydb/backup/archive | head -2  # Newest
```

#### Archive Log Cleanup

```sql
-- Delete archive logs older than 7 days
-- (Ensure backups are complete before deleting!)
DELETE FROM v$archived_log
WHERE completion_time < NOW() - INTERVAL '7 days'
  AND applied = TRUE
  AND backed_up = TRUE;

-- Or use RMAN-like command
EXEC delete_archived_logs(older_than_days => 7, backed_up => true);
```

#### Automated Archive Log Management

```bash
#!/bin/bash
# /usr/local/bin/rustydb-archive-cleanup.sh

ARCHIVE_DIR="/var/lib/rustydb/backup/archive"
RETENTION_DAYS=7

# Delete archive logs older than retention period
find "$ARCHIVE_DIR" -name "*.wal" -mtime +$RETENTION_DAYS -delete

# Log cleanup
echo "[$(date)] Archive log cleanup completed" >> /var/log/rustydb/maintenance.log

# Verify sufficient disk space remains
DISK_USAGE=$(df -h "$ARCHIVE_DIR" | awk 'NR==2 {print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -gt 85 ]; then
    echo "WARNING: Archive disk usage is ${DISK_USAGE}%" | \
        mail -s "RustyDB Archive Disk Warning" dba@company.com
fi
```

Schedule in crontab:
```bash
# Run daily at 1 AM
0 1 * * * /usr/local/bin/rustydb-archive-cleanup.sh
```

---

## 5. Memory Administration

### 5.1 Memory Architecture

RustyDB memory is divided into several key areas:

```
Memory Layout:
┌─────────────────────────────────────────────────────────┐
│                  Total System Memory                     │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌────────────────────────────────────────────────┐     │
│  │        RustyDB Memory (configured)              │     │
│  ├────────────────────────────────────────────────┤     │
│  │                                                 │     │
│  │  Buffer Pool (75%)      ┌──────────────────┐   │     │
│  │  ├─ Data pages          │  8 GB            │   │     │
│  │  ├─ Index pages         │                  │   │     │
│  │  └─ Query results       │                  │   │     │
│  │                         └──────────────────┘   │     │
│  │                                                 │     │
│  │  Shared Memory (15%)    ┌──────────────────┐   │     │
│  │  ├─ Connection state    │  1.6 GB          │   │     │
│  │  ├─ Lock tables         │                  │   │     │
│  │  └─ System catalog      │                  │   │     │
│  │                         └──────────────────┘   │     │
│  │                                                 │     │
│  │  WAL Buffer (5%)        ┌──────────────────┐   │     │
│  │  └─ WAL records         │  512 MB          │   │     │
│  │                         └──────────────────┘   │     │
│  │                                                 │     │
│  │  Query Cache (5%)       ┌──────────────────┐   │     │
│  │  └─ Cached results      │  512 MB          │   │     │
│  │                         └──────────────────┘   │     │
│  └────────────────────────────────────────────────┘     │
│                                                          │
│  OS and Other Processes (remaining memory)               │
└─────────────────────────────────────────────────────────┘
```

### 5.2 Buffer Pool Management

The buffer pool caches frequently accessed data pages in memory.

#### View Buffer Pool Statistics

```sql
-- Buffer pool overview
SELECT
    total_pages,
    used_pages,
    dirty_pages,
    free_pages,
    ROUND(hit_ratio * 100, 2) as hit_ratio_percent,
    reads,
    writes,
    evictions
FROM v$buffer_pool_stats;
```

Example output:
```
total_pages | used_pages | dirty_pages | free_pages | hit_ratio_percent | reads    | writes   | evictions
-----------+------------+-------------+------------+-------------------+----------+----------+-----------
2097152    | 2050000    | 125000      | 47152      | 98.75             | 50000000 | 15000000 | 2500000
```

#### Monitor Buffer Pool Hit Ratio

**Target:** > 95% for OLTP workloads, > 90% for mixed workloads

```sql
-- Real-time buffer pool hit ratio
SELECT
    ROUND((cache_hits * 100.0) / NULLIF(cache_hits + cache_misses, 0), 2) as hit_ratio,
    cache_hits,
    cache_misses
FROM (
    SELECT
        SUM(buffer_hits) as cache_hits,
        SUM(buffer_misses) as cache_misses
    FROM v$buffer_cache_stats
) stats;
```

#### Tune Buffer Pool Size

```sql
-- Check recommended buffer pool size
SELECT
    current_size_mb,
    recommended_size_mb,
    CASE
        WHEN recommended_size_mb > current_size_mb THEN 'INCREASE'
        WHEN recommended_size_mb < current_size_mb THEN 'DECREASE'
        ELSE 'OK'
    END as recommendation,
    reason
FROM v$buffer_pool_advisor;
```

**Increase buffer pool:**

```toml
# /etc/rustydb/config.toml
[memory]
buffer_pool_size = 17179869184  # Increase from 8 GB to 16 GB

# Restart required
```

#### Buffer Pool Content Analysis

```sql
-- View top objects in buffer pool
SELECT
    object_type,
    object_name,
    buffer_count,
    ROUND(buffer_count * 100.0 / total_buffers, 2) as percent_of_pool
FROM (
    SELECT
        object_type,
        object_name,
        COUNT(*) as buffer_count,
        (SELECT COUNT(*) FROM v$buffer_cache) as total_buffers
    FROM v$buffer_cache
    GROUP BY object_type, object_name
) t
ORDER BY buffer_count DESC
LIMIT 20;
```

#### Flush Buffer Pool

```sql
-- Flush dirty buffers to disk
ALTER SYSTEM CHECKPOINT;

-- Or via API
curl -X POST http://localhost:8080/api/v1/admin/checkpoint \
  -H "Authorization: Bearer ${ADMIN_TOKEN}"
```

### 5.3 Memory Allocation and Monitoring

#### View Memory Usage by Component

```sql
-- Memory usage by component
SELECT
    component,
    allocated_bytes,
    used_bytes,
    ROUND(allocated_bytes / 1024.0 / 1024, 2) as allocated_mb,
    ROUND(used_bytes / 1024.0 / 1024, 2) as used_mb,
    ROUND(used_bytes * 100.0 / NULLIF(allocated_bytes, 0), 2) as usage_percent
FROM v$memory_usage
ORDER BY allocated_bytes DESC;
```

Example output:
```
component       | allocated_mb | used_mb | usage_percent
----------------+--------------+---------+--------------
buffer_pool     | 8192.00      | 7895.50 | 96.38
shared_memory   | 2048.00      | 1567.25 | 76.53
wal_buffer      | 64.00        | 45.30   | 70.78
query_cache     | 512.00       | 384.75  | 75.15
connection_pool | 256.00       | 178.90  | 69.88
```

#### Monitor Session Memory Usage

```sql
-- Top sessions by memory usage
SELECT
    session_id,
    user_name,
    state,
    ROUND(pga_allocated / 1024.0 / 1024, 2) as pga_mb,
    ROUND(temp_space / 1024.0 / 1024, 2) as temp_mb,
    current_query
FROM v$session
WHERE state = 'ACTIVE'
ORDER BY pga_allocated DESC
LIMIT 10;
```

#### Set Memory Limits per Session

```sql
-- Set memory limit for current session
ALTER SESSION SET work_mem = '128MB';

-- Set memory limit for user
ALTER USER app_user SET work_mem = '256MB';

-- View memory quotas
SELECT username, work_mem, maintenance_work_mem
FROM v$user_quotas;
```

### 5.4 Work Memory Configuration

Work memory is used for sort operations, hash joins, and other in-memory operations.

#### Configure Work Memory

```toml
# /etc/rustydb/config.toml
[memory]
# Default buffer pool: 1000 pages × 8192 bytes = ~8 MB
# Scale based on workload (production typically uses 25-40% of RAM)
work_mem = 67108864              # 64 MB per operation (default)
maintenance_work_mem = 2147483648 # 2 GB for maintenance operations
```

#### Monitor Work Memory Usage

```sql
-- View sort/hash operations using work memory
SELECT
    query_id,
    operation_type,
    allocated_bytes,
    disk_used_bytes,
    CASE
        WHEN disk_used_bytes > 0 THEN 'DISK'
        ELSE 'MEMORY'
    END as execution_mode
FROM v$work_memory_usage
WHERE operation_time > NOW() - INTERVAL '1 hour'
ORDER BY allocated_bytes DESC;
```

**Note:** If many operations spill to disk, increase work_mem.

#### Tune Work Memory

**Guidelines:**
- OLTP: 64-128 MB
- Mixed: 128-256 MB
- OLAP: 256-512 MB or higher
- Data Warehouse: 512 MB - 2 GB

```sql
-- For OLAP queries, increase work memory
ALTER SESSION SET work_mem = '512MB';

-- Run memory-intensive query
SELECT /*+ PARALLEL(8) */ ...
```

### 5.5 Memory Optimization

#### Enable Huge Pages (Linux)

Huge pages reduce TLB (Translation Lookaside Buffer) misses for large memory allocations.

```bash
# Calculate huge pages needed (2 MB pages)
# For 16 GB buffer pool: 16384 MB / 2 MB = 8192 pages

# Configure huge pages
echo 8192 > /proc/sys/vm/nr_hugepages

# Make persistent
echo "vm.nr_hugepages = 8192" >> /etc/sysctl.conf
sysctl -p
```

Enable in RustyDB:
```toml
[memory]
use_huge_pages = true
```

#### Memory Compaction

```sql
-- Run memory compaction (reduces fragmentation)
ALTER SYSTEM COMPACT MEMORY;

-- View memory fragmentation
SELECT
    component,
    fragmentation_percent,
    CASE
        WHEN fragmentation_percent > 30 THEN 'HIGH'
        WHEN fragmentation_percent > 15 THEN 'MEDIUM'
        ELSE 'LOW'
    END as fragmentation_level
FROM v$memory_fragmentation;
```

#### Automatic Memory Management

```toml
[memory]
# Enable automatic memory tuning
adaptive_memory_enabled = true

# Allow up to 20% variance from configured values
adaptive_memory_max_variance_percent = 20
```

---

## 6. User and Schema Management

### 6.1 User Administration

#### Create Users

```sql
-- Create basic user
CREATE USER app_user
    IDENTIFIED BY 'Secure$Password123'
    DEFAULT TABLESPACE users
    TEMPORARY TABLESPACE temp
    QUOTA 10G ON users;

-- Create user with all options
CREATE USER reporting_user
    IDENTIFIED BY 'Reporting$Pass456'
    DEFAULT TABLESPACE app_data
    TEMPORARY TABLESPACE app_temp
    QUOTA 50G ON app_data
    PROFILE olap_profile
    ACCOUNT UNLOCK
    PASSWORD EXPIRE;
```

#### View Users

```sql
-- List all users
SELECT
    username,
    account_status,
    default_tablespace,
    created,
    profile,
    last_login
FROM dba_users
ORDER BY username;

-- View current user
SELECT CURRENT_USER;

-- View user privileges
SELECT * FROM dba_sys_privs WHERE grantee = 'APP_USER';

-- View user roles
SELECT * FROM dba_role_privs WHERE grantee = 'APP_USER';
```

#### Modify Users

```sql
-- Change password
ALTER USER app_user IDENTIFIED BY 'NewPassword$789';

-- Force password change on next login
ALTER USER app_user PASSWORD EXPIRE;

-- Change default tablespace
ALTER USER app_user DEFAULT TABLESPACE new_tablespace;

-- Change quota
ALTER USER app_user QUOTA 20G ON users;
ALTER USER app_user QUOTA UNLIMITED ON users;

-- Lock user account
ALTER USER app_user ACCOUNT LOCK;

-- Unlock user account
ALTER USER app_user ACCOUNT UNLOCK;
```

#### Drop Users

```sql
-- Drop user (fails if user owns objects)
DROP USER app_user;

-- Drop user and all owned objects
DROP USER app_user CASCADE;
```

### 6.2 Role Management

Roles simplify privilege management by grouping privileges together.

#### Create Roles

```sql
-- Create basic role
CREATE ROLE app_developer;

-- Create role with password
CREATE ROLE secure_role IDENTIFIED BY 'RolePassword123';
```

#### Grant Privileges to Roles

```sql
-- Grant system privileges
GRANT CREATE SESSION TO app_developer;
GRANT CREATE TABLE TO app_developer;
GRANT CREATE VIEW TO app_developer;
GRANT CREATE PROCEDURE TO app_developer;

-- Grant object privileges
GRANT SELECT, INSERT, UPDATE, DELETE ON sales TO app_developer;
GRANT EXECUTE ON process_order TO app_developer;

-- Grant role to role
GRANT app_developer TO senior_developer;
```

#### Grant Roles to Users

```sql
-- Grant role to user
GRANT app_developer TO alice;
GRANT senior_developer TO bob;

-- Grant with ADMIN OPTION (user can grant role to others)
GRANT app_developer TO charlie WITH ADMIN OPTION;
```

#### Revoke Privileges and Roles

```sql
-- Revoke system privilege
REVOKE CREATE TABLE FROM app_developer;

-- Revoke object privilege
REVOKE DELETE ON sales FROM app_developer;

-- Revoke role from user
REVOKE app_developer FROM alice;
```

#### View Roles and Privileges

```sql
-- List all roles
SELECT * FROM dba_roles ORDER BY role;

-- View role privileges
SELECT * FROM dba_sys_privs WHERE grantee = 'APP_DEVELOPER';

-- View users with specific role
SELECT * FROM dba_role_privs WHERE granted_role = 'APP_DEVELOPER';

-- View current user's roles
SELECT * FROM session_roles;
```

#### Enable/Disable Roles in Session

```sql
-- Disable role for current session
SET ROLE NONE;

-- Enable specific roles
SET ROLE app_developer, app_reader;

-- Enable all roles
SET ROLE ALL;
```

### 6.3 Privilege Management

RustyDB has two types of privileges: **System Privileges** and **Object Privileges**.

#### System Privileges

System privileges allow users to perform database-wide operations.

**Common System Privileges:**

| Privilege | Description |
|-----------|-------------|
| CREATE SESSION | Connect to the database |
| CREATE TABLE | Create tables in own schema |
| CREATE VIEW | Create views |
| CREATE PROCEDURE | Create stored procedures |
| CREATE SEQUENCE | Create sequences |
| CREATE USER | Create database users |
| ALTER USER | Modify user accounts |
| DROP USER | Drop user accounts |
| GRANT ANY PRIVILEGE | Grant any privilege to others |
| RESTRICTED SESSION | Connect during restricted mode |
| SYSDBA | Full administrative privileges |

```sql
-- Grant system privileges
GRANT CREATE SESSION TO app_user;
GRANT CREATE TABLE, CREATE VIEW TO app_user;
GRANT SYSDBA TO admin_user;
```

#### Object Privileges

Object privileges allow users to perform operations on specific database objects.

**Common Object Privileges:**

| Privilege | Applies To | Description |
|-----------|------------|-------------|
| SELECT | Tables, Views | Query data |
| INSERT | Tables | Insert rows |
| UPDATE | Tables | Update rows |
| DELETE | Tables | Delete rows |
| EXECUTE | Procedures, Functions | Execute code |
| REFERENCES | Tables | Create foreign keys |
| INDEX | Tables | Create indexes |

```sql
-- Grant object privileges
GRANT SELECT ON employees TO app_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON orders TO app_user;
GRANT EXECUTE ON calculate_total TO app_user;

-- Grant all privileges on object
GRANT ALL PRIVILEGES ON sales TO app_user;

-- Grant with GRANT OPTION (user can grant to others)
GRANT SELECT ON employees TO app_user WITH GRANT OPTION;
```

#### Schema Privileges

```sql
-- Grant all privileges on all tables in schema
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO app_user;

-- Grant specific privilege on all tables
GRANT SELECT ON ALL TABLES IN SCHEMA reporting TO analyst_user;

-- Grant on future tables (automatic grants)
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_user;
```

### 6.4 Profile Management

Profiles control resource limits and password policies for users.

#### Create Profile

```sql
-- Create profile with resource limits
CREATE PROFILE app_profile LIMIT
    SESSIONS_PER_USER 5
    CPU_PER_SESSION 600000              -- 10 minutes (in milliseconds)
    CPU_PER_CALL 60000                  -- 1 minute
    CONNECT_TIME UNLIMITED
    IDLE_TIME 30                        -- 30 minutes
    LOGICAL_READS_PER_SESSION 100000
    LOGICAL_READS_PER_CALL 10000
    PRIVATE_MEMORY 50M
    COMPOSITE_LIMIT UNLIMITED;

-- Create profile with password policy
CREATE PROFILE secure_profile LIMIT
    FAILED_LOGIN_ATTEMPTS 3
    PASSWORD_LIFE_TIME 90               -- Days
    PASSWORD_REUSE_TIME 365             -- Days
    PASSWORD_REUSE_MAX 5                -- Cannot reuse last 5 passwords
    PASSWORD_LOCK_TIME 1                -- Days
    PASSWORD_GRACE_TIME 7               -- Days warning before expiration
    PASSWORD_VERIFY_FUNCTION verify_password;
```

#### Assign Profile to User

```sql
-- Assign profile to user
ALTER USER app_user PROFILE app_profile;

-- View user's profile
SELECT username, profile FROM dba_users WHERE username = 'APP_USER';
```

#### View Profile Limits

```sql
-- View all profiles and their limits
SELECT
    profile,
    resource_name,
    limit
FROM dba_profiles
ORDER BY profile, resource_name;

-- View specific profile
SELECT resource_name, limit
FROM dba_profiles
WHERE profile = 'APP_PROFILE';
```

#### Modify Profile

```sql
-- Modify profile limits
ALTER PROFILE app_profile LIMIT
    SESSIONS_PER_USER 10
    IDLE_TIME 60;
```

#### Drop Profile

```sql
-- Drop profile (fails if assigned to users)
DROP PROFILE app_profile;

-- Drop profile and reassign users to default
DROP PROFILE app_profile CASCADE;
```

### 6.5 Schema Management

A schema is a collection of database objects owned by a user.

#### Create Schema

```sql
-- Schema is automatically created when user is created
CREATE USER app_schema
    IDENTIFIED BY 'Password123'
    DEFAULT TABLESPACE app_data;

-- Grant privileges to create objects
GRANT CREATE TABLE, CREATE VIEW, CREATE PROCEDURE TO app_schema;
```

#### View Schema Objects

```sql
-- List all objects in schema
SELECT
    object_type,
    object_name,
    created,
    last_ddl_time,
    status
FROM dba_objects
WHERE owner = 'APP_SCHEMA'
ORDER BY object_type, object_name;

-- Count objects by type
SELECT
    object_type,
    COUNT(*) as object_count
FROM dba_objects
WHERE owner = 'APP_SCHEMA'
GROUP BY object_type
ORDER BY object_count DESC;
```

#### Move Objects Between Schemas

```sql
-- Change table ownership
ALTER TABLE old_schema.my_table OWNER TO new_schema;

-- Export/import schema (using backup tools)
rusty-db-backup export --schema app_schema --file /tmp/app_schema.dump
rusty-db-backup import --schema new_schema --file /tmp/app_schema.dump
```

#### Drop Schema

```sql
-- Drop schema and all objects
DROP USER app_schema CASCADE;
```

---

## 7. Space Management

### 7.1 Monitoring Space Usage

#### Database-Wide Space Usage

```sql
-- Total database size
SELECT
    ROUND(SUM(total_size_bytes) / 1024.0 / 1024 / 1024, 2) as total_db_size_gb
FROM v$tablespace;

-- Space usage by tablespace
SELECT
    tablespace_name,
    ROUND(total_size_mb / 1024.0, 2) as total_gb,
    ROUND(used_size_mb / 1024.0, 2) as used_gb,
    ROUND(free_size_mb / 1024.0, 2) as free_gb,
    ROUND(used_size_mb * 100.0 / total_size_mb, 2) as usage_percent
FROM v$tablespace
ORDER BY usage_percent DESC;
```

#### Table Space Usage

```sql
-- Top 20 tables by size
SELECT
    schema_name,
    table_name,
    ROUND(total_size_bytes / 1024.0 / 1024, 2) as size_mb,
    row_count,
    ROUND(total_size_bytes / NULLIF(row_count, 0), 2) as bytes_per_row
FROM v$table_statistics
ORDER BY total_size_bytes DESC
LIMIT 20;

-- Tables with high growth rate
SELECT
    schema_name,
    table_name,
    size_mb,
    growth_mb_per_day,
    CASE
        WHEN growth_mb_per_day > 1000 THEN 'HIGH'
        WHEN growth_mb_per_day > 100 THEN 'MEDIUM'
        ELSE 'LOW'
    END as growth_rate
FROM v$table_growth
WHERE growth_mb_per_day > 0
ORDER BY growth_mb_per_day DESC;
```

#### Index Space Usage

```sql
-- Index sizes
SELECT
    schema_name,
    table_name,
    index_name,
    ROUND(size_bytes / 1024.0 / 1024, 2) as size_mb,
    index_type,
    is_unique
FROM v$index_statistics
ORDER BY size_bytes DESC
LIMIT 20;

-- Index vs. table size ratio
SELECT
    t.schema_name,
    t.table_name,
    ROUND(t.size_mb, 2) as table_size_mb,
    ROUND(i.total_index_size_mb, 2) as index_size_mb,
    ROUND(i.total_index_size_mb * 100.0 / NULLIF(t.size_mb, 0), 2) as index_to_table_ratio
FROM (
    SELECT schema_name, table_name,
           SUM(size_bytes) / 1024.0 / 1024 as size_mb
    FROM v$table_statistics
    GROUP BY schema_name, table_name
) t
JOIN (
    SELECT schema_name, table_name,
           SUM(size_bytes) / 1024.0 / 1024 as total_index_size_mb
    FROM v$index_statistics
    GROUP BY schema_name, table_name
) i USING (schema_name, table_name)
WHERE i.total_index_size_mb > 100
ORDER BY index_to_table_ratio DESC;
```

#### WAL and Archive Log Space

```bash
# Check WAL directory size
du -sh /var/lib/rustydb/wal

# Check archive log directory size
du -sh /var/lib/rustydb/backup/archive

# List archive logs with sizes
ls -lh /var/lib/rustydb/backup/archive | tail -20
```

```sql
-- WAL statistics
SELECT
    ROUND(wal_size_mb, 2) as current_wal_size_mb,
    ROUND(wal_generated_mb_per_hour, 2) as generation_rate_mb_per_hour,
    ROUND(wal_size_mb / NULLIF(wal_generated_mb_per_hour, 0), 2) as hours_of_wal
FROM v$wal_statistics;
```

### 7.2 Reclaiming Space

#### Vacuum (Space Reclamation)

VACUUM reclaims storage space from deleted or obsolete rows.

```sql
-- Vacuum specific table
VACUUM table_name;

-- Vacuum with statistics update
VACUUM ANALYZE table_name;

-- Vacuum entire database
VACUUM;

-- Full vacuum (rewrites table, more aggressive)
VACUUM FULL table_name;
```

**Note:** FULL vacuum requires exclusive lock and may take significant time.

#### View Bloat Statistics

```sql
-- Tables with high bloat
SELECT
    schema_name,
    table_name,
    ROUND(live_size_mb, 2) as live_mb,
    ROUND(dead_size_mb, 2) as dead_mb,
    ROUND(dead_size_mb * 100.0 / NULLIF(live_size_mb + dead_size_mb, 0), 2) as bloat_percent
FROM v$table_bloat
WHERE dead_size_mb > 100
ORDER BY bloat_percent DESC;
```

#### Truncate Tables

For large deletions, TRUNCATE is faster than DELETE.

```sql
-- Truncate table (fast, cannot be rolled back)
TRUNCATE TABLE staging_table;

-- Truncate and reset sequences
TRUNCATE TABLE orders RESTART IDENTITY;

-- Truncate with cascade (also truncates referencing tables)
TRUNCATE TABLE parent_table CASCADE;
```

**TRUNCATE vs. DELETE:**

| Operation | TRUNCATE | DELETE |
|-----------|----------|--------|
| Speed | Fast (drops data files) | Slow (row-by-row) |
| Rollback | No | Yes |
| Triggers | Not fired | Fired |
| Space | Immediately released | Released after VACUUM |
| Constraints | Must drop foreign keys | Respects foreign keys |

#### Drop Unused Indexes

```sql
-- Find unused indexes
SELECT
    schema_name,
    table_name,
    index_name,
    ROUND(size_mb, 2) as size_mb,
    last_used,
    EXTRACT(DAYS FROM (NOW() - last_used)) as days_unused
FROM v$index_usage
WHERE last_used < NOW() - INTERVAL '90 days'
   OR last_used IS NULL
ORDER BY size_mb DESC;

-- Drop unused index
DROP INDEX schema_name.unused_index;
```

#### Compress Tables

```sql
-- Enable compression on table
ALTER TABLE large_table SET COMPRESSION = 'zstd';

-- Rewrite table to apply compression
VACUUM FULL large_table;

-- Check compression ratio
SELECT
    table_name,
    ROUND(uncompressed_size_mb, 2) as uncompressed_mb,
    ROUND(compressed_size_mb, 2) as compressed_mb,
    ROUND((1 - compressed_size_mb / NULLIF(uncompressed_size_mb, 0)) * 100, 2) as compression_ratio_percent
FROM v$table_compression;
```

### 7.3 Segment Management

#### View Segments

```sql
-- List all segments
SELECT
    segment_type,
    segment_name,
    tablespace_name,
    ROUND(bytes / 1024.0 / 1024, 2) as size_mb,
    extents,
    blocks
FROM dba_segments
ORDER BY bytes DESC
LIMIT 50;

-- View segment growth
SELECT
    segment_name,
    segment_type,
    ROUND(size_mb, 2) as current_size_mb,
    ROUND(growth_mb_per_month, 2) as monthly_growth_mb,
    ROUND(size_mb + (growth_mb_per_month * 6), 2) as projected_size_6mo_mb
FROM v$segment_growth
WHERE growth_mb_per_month > 10
ORDER BY growth_mb_per_month DESC;
```

#### Shrink Segments

```sql
-- Shrink table segment (online operation)
ALTER TABLE large_table SHRINK SPACE;

-- Shrink table and dependent objects (indexes)
ALTER TABLE large_table SHRINK SPACE CASCADE;

-- Shrink index segment
ALTER INDEX large_index SHRINK SPACE;
```

#### Reorganize Tables

```sql
-- Reorganize table (reorders rows, rebuilds indexes)
ALTER TABLE fragmented_table REORGANIZE;

-- Move table to new tablespace
ALTER TABLE my_table MOVE TABLESPACE new_tablespace;

-- Rebuild all indexes after move
SELECT 'ALTER INDEX ' || index_name || ' REBUILD;'
FROM dba_indexes
WHERE table_name = 'MY_TABLE';
```

### 7.4 Capacity Planning

#### Forecast Storage Growth

```bash
#!/bin/bash
# /usr/local/bin/rustydb-capacity-forecast.sh

rusty-db-cli << EOF
-- Generate 6-month capacity forecast
SELECT
    tablespace_name,
    ROUND(current_size_gb, 2) as current_gb,
    ROUND(growth_rate_gb_per_month, 2) as monthly_growth_gb,
    ROUND(current_size_gb + (growth_rate_gb_per_month * 1), 2) as forecast_1mo_gb,
    ROUND(current_size_gb + (growth_rate_gb_per_month * 3), 2) as forecast_3mo_gb,
    ROUND(current_size_gb + (growth_rate_gb_per_month * 6), 2) as forecast_6mo_gb,
    CASE
        WHEN (current_size_gb + growth_rate_gb_per_month * 6) > (max_size_gb * 0.8) THEN 'ACTION REQUIRED'
        WHEN (current_size_gb + growth_rate_gb_per_month * 6) > (max_size_gb * 0.7) THEN 'MONITOR'
        ELSE 'OK'
    END as status
FROM v$tablespace_forecast
ORDER BY growth_rate_gb_per_month DESC;
EOF
```

#### Set Up Capacity Alerts

```sql
-- Create capacity monitoring alert
EXEC configure_alert(
    name => 'tablespace_capacity_warning',
    metric => 'tablespace_usage_percent',
    threshold => 85.0,
    severity => 'WARNING',
    action => 'NOTIFY_DBA'
);

EXEC configure_alert(
    name => 'tablespace_capacity_critical',
    metric => 'tablespace_usage_percent',
    threshold => 95.0,
    severity => 'CRITICAL',
    action => 'PAGE_DBA'
);
```

#### Automated Space Management

```toml
# /etc/rustydb/config.toml

[storage]
# Automatic tablespace extension
auto_extend_enabled = true
auto_extend_threshold_percent = 85
auto_extend_size_mb = 1024

# Automatic cleanup
auto_cleanup_enabled = true
auto_cleanup_threshold_percent = 90
```

---

## 8. Routine Maintenance

### 8.1 Daily Maintenance Tasks

#### Daily Checklist

```bash
#!/bin/bash
# /usr/local/bin/rustydb-daily-maintenance.sh

echo "=== RustyDB Daily Maintenance - $(date) ==="

# 1. Check database status
echo "Checking database status..."
systemctl status rustydb | grep Active

# 2. Check for errors in logs
echo "Checking for errors..."
ERROR_COUNT=$(grep -i error /var/log/rustydb/rustydb.log | wc -l)
echo "Errors in last 24 hours: $ERROR_COUNT"
if [ "$ERROR_COUNT" -gt 100 ]; then
    echo "WARNING: High error count detected"
fi

# 3. Check disk space
echo "Checking disk space..."
df -h /var/lib/rustydb

# 4. Check replication lag (if applicable)
echo "Checking replication..."
rusty-db-cli -c "SELECT replica_name, lag_seconds FROM v$replication_status"

# 5. Check backup status
echo "Checking backup status..."
LAST_BACKUP=$(ls -t /var/lib/rustydb/backup/*.tar.gz 2>/dev/null | head -1)
if [ -n "$LAST_BACKUP" ]; then
    echo "Last backup: $LAST_BACKUP"
    BACKUP_AGE=$(find "$LAST_BACKUP" -mtime +1 | wc -l)
    if [ "$BACKUP_AGE" -gt 0 ]; then
        echo "WARNING: Last backup is more than 24 hours old"
    fi
else
    echo "WARNING: No backups found"
fi

# 6. Check for blocking sessions
echo "Checking for blocking sessions..."
BLOCKED_COUNT=$(rusty-db-cli -t -c "SELECT COUNT(*) FROM v$session WHERE blocking_session IS NOT NULL")
echo "Blocked sessions: $BLOCKED_COUNT"

# 7. Review alert log
echo "Recent alerts:"
rusty-db-cli -c "SELECT severity, name, message FROM v$alerts WHERE occurred_at > NOW() - INTERVAL '24 hours' ORDER BY occurred_at DESC LIMIT 5"

# 8. Check long-running queries
echo "Long-running queries:"
rusty-db-cli -c "SELECT query_id, EXTRACT(EPOCH FROM elapsed_time) as seconds, LEFT(sql_text, 80) FROM v$sql_monitor WHERE elapsed_time > INTERVAL '5 minutes' ORDER BY elapsed_time DESC LIMIT 5"

echo "=== Daily Maintenance Complete ==="
```

Schedule in crontab:
```bash
# Run daily at 8 AM
0 8 * * * /usr/local/bin/rustydb-daily-maintenance.sh | mail -s "RustyDB Daily Report" dba@company.com
```

#### Monitor Active Sessions

```sql
-- Daily session review
SELECT
    user_name,
    COUNT(*) as session_count,
    SUM(CASE WHEN state = 'ACTIVE' THEN 1 ELSE 0 END) as active_count,
    SUM(CASE WHEN state = 'IDLE' THEN 1 ELSE 0 END) as idle_count
FROM v$session
WHERE user_name NOT IN ('system', 'rustydb')
GROUP BY user_name
ORDER BY session_count DESC;
```

#### Review Query Performance

```sql
-- Daily query performance report
SELECT
    LEFT(sql_text, 100) as query,
    executions,
    ROUND(total_time_ms / 1000.0, 2) as total_time_sec,
    ROUND(avg_time_ms, 2) as avg_time_ms,
    ROUND(max_time_ms, 2) as max_time_ms
FROM v$sql_statistics
WHERE first_seen > NOW() - INTERVAL '24 hours'
ORDER BY total_time_ms DESC
LIMIT 20;
```

### 8.2 Weekly Maintenance Tasks

#### Weekly Checklist

```bash
#!/bin/bash
# /usr/local/bin/rustydb-weekly-maintenance.sh

echo "=== RustyDB Weekly Maintenance - $(date) ==="

# 1. Update statistics
echo "Updating table statistics..."
rusty-db-cli << EOF
ANALYZE;
EOF

# 2. Vacuum database
echo "Running VACUUM..."
rusty-db-cli << EOF
VACUUM ANALYZE;
EOF

# 3. Rebuild fragmented indexes
echo "Checking index fragmentation..."
rusty-db-cli << EOF
SELECT schema_name, table_name, index_name, fragmentation_percent
FROM v$index_fragmentation
WHERE fragmentation_percent > 30
ORDER BY fragmentation_percent DESC;
EOF

# 4. Clean up archive logs
echo "Cleaning up old archive logs..."
/usr/local/bin/rustydb-archive-cleanup.sh

# 5. Review tablespace usage
echo "Tablespace usage:"
rusty-db-cli << EOF
SELECT tablespace_name,
       ROUND(used_size_mb * 100.0 / total_size_mb, 2) as usage_percent
FROM v$tablespace
WHERE ROUND(used_size_mb * 100.0 / total_size_mb, 2) > 75
ORDER BY usage_percent DESC;
EOF

# 6. Check for unused indexes
echo "Checking for unused indexes..."
rusty-db-cli << EOF
SELECT schema_name, table_name, index_name,
       ROUND(size_mb, 2) as size_mb,
       EXTRACT(DAYS FROM (NOW() - last_used)) as days_unused
FROM v$index_usage
WHERE last_used < NOW() - INTERVAL '90 days'
ORDER BY size_mb DESC
LIMIT 10;
EOF

# 7. Performance report
echo "Generating weekly performance report..."
rusty-db-cli << EOF
SELECT
    metric_name,
    ROUND(avg_value, 2) as avg_value,
    ROUND(max_value, 2) as max_value,
    ROUND(min_value, 2) as min_value
FROM v$weekly_metrics
WHERE week = DATE_TRUNC('week', NOW())
ORDER BY metric_name;
EOF

echo "=== Weekly Maintenance Complete ==="
```

Schedule in crontab:
```bash
# Run weekly on Sunday at 3 AM
0 3 * * 0 /usr/local/bin/rustydb-weekly-maintenance.sh | mail -s "RustyDB Weekly Report" dba@company.com
```

### 8.3 Monthly Maintenance Tasks

#### Monthly Checklist

```bash
#!/bin/bash
# /usr/local/bin/rustydb-monthly-maintenance.sh

echo "=== RustyDB Monthly Maintenance - $(date) ==="

# 1. Full database statistics
echo "Collecting comprehensive statistics..."
rusty-db-cli << EOF
EXEC collect_statistics(scope => 'DATABASE', estimate_percent => 100);
EOF

# 2. Rebuild heavily fragmented indexes
echo "Rebuilding fragmented indexes..."
rusty-db-cli << EOF
SELECT 'ALTER INDEX ' || schema_name || '.' || index_name || ' REBUILD;'
FROM v$index_fragmentation
WHERE fragmentation_percent > 40;
EOF

# 3. Archive old data
echo "Archiving old data..."
# Run data archiving procedures
rusty-db-cli << EOF
EXEC archive_old_orders(older_than_days => 365);
EXEC archive_old_logs(older_than_days => 180);
EOF

# 4. Capacity planning report
echo "Generating capacity planning report..."
/usr/local/bin/rustydb-capacity-forecast.sh

# 5. Security audit
echo "Running security audit..."
rusty-db-cli << EOF
-- Users with admin privileges
SELECT username, granted_role, admin_option
FROM dba_role_privs
WHERE granted_role IN ('SYSDBA', 'SYSOPER');

-- Users with password about to expire
SELECT username, password_expiry_date,
       EXTRACT(DAYS FROM (password_expiry_date - NOW())) as days_until_expiry
FROM dba_users
WHERE password_expiry_date < NOW() + INTERVAL '30 days'
ORDER BY password_expiry_date;
EOF

# 6. Review backup strategy
echo "Reviewing backups..."
ls -lh /var/lib/rustydb/backup/*.tar.gz | tail -10

# 7. Test backup restore (to test environment)
echo "Testing backup restore..."
LATEST_BACKUP=$(ls -t /var/lib/rustydb/backup/*.tar.gz | head -1)
echo "Latest backup: $LATEST_BACKUP"
# Perform test restore to test instance (not shown)

# 8. Update documentation
echo "Reminder: Update system documentation with any changes"

# 9. Performance baseline
echo "Capturing performance baseline..."
rusty-db-cli << EOF
EXEC capture_snapshot();
EXEC create_baseline(
    name => 'Monthly_' || TO_CHAR(NOW(), 'YYYY_MM'),
    baseline_type => 'STATIC'
);
EOF

echo "=== Monthly Maintenance Complete ==="
```

Schedule in crontab:
```bash
# Run monthly on the 1st at 2 AM
0 2 1 * * /usr/local/bin/rustydb-monthly-maintenance.sh | mail -s "RustyDB Monthly Report" dba@company.com
```

### 8.4 Automated Maintenance Configuration

#### Enable Auto-Maintenance

```toml
# /etc/rustydb/config.toml

[maintenance]
# Automatic VACUUM
auto_vacuum_enabled = true
vacuum_schedule = "0 3 * * 0"  # 3 AM every Sunday
vacuum_threshold_percent = 20   # Vacuum if >20% dead rows

# Automatic ANALYZE
auto_analyze_enabled = true
analyze_schedule = "0 4 * * *"  # 4 AM daily
analyze_threshold_percent = 10  # Analyze if >10% rows changed

# Automatic checkpoint
checkpoint_interval_sec = 300   # 5 minutes

# Index maintenance
auto_reindex_enabled = true
reindex_fragmentation_threshold = 40  # Rebuild if >40% fragmented
```

#### Monitor Auto-Maintenance Jobs

```sql
-- View maintenance job history
SELECT
    job_name,
    start_time,
    end_time,
    status,
    EXTRACT(EPOCH FROM (end_time - start_time)) as duration_seconds,
    details
FROM v$maintenance_jobs
WHERE start_time > NOW() - INTERVAL '7 days'
ORDER BY start_time DESC;

-- View current auto-maintenance settings
SELECT
    parameter,
    value,
    enabled
FROM v$auto_maintenance_config;
```

---

## 9. Job Scheduling

### 9.1 Background Job System

RustyDB includes a built-in job scheduler for routine tasks.

#### Create Job

```sql
-- Create daily backup job
CREATE JOB daily_backup
    SCHEDULE 'AT 02:00'
    EXEC PROCEDURE backup_database();

-- Create weekly maintenance job
CREATE JOB weekly_maintenance
    SCHEDULE 'AT 03:00 ON SUNDAY'
    EXEC PROCEDURE weekly_maintenance_tasks();

-- Create job with cron syntax
CREATE JOB hourly_stats
    SCHEDULE 'CRON 0 * * * *'  -- Every hour
    EXEC SQL 'EXEC collect_statistics(scope => ''INCREMENTAL'')';
```

#### View Jobs

```sql
-- List all jobs
SELECT
    job_id,
    job_name,
    schedule,
    last_run,
    next_run,
    status,
    enabled
FROM dba_jobs
ORDER BY next_run;

-- View job history
SELECT
    job_name,
    start_time,
    end_time,
    status,
    error_message
FROM dba_job_history
WHERE start_time > NOW() - INTERVAL '7 days'
ORDER BY start_time DESC;
```

#### Modify Job

```sql
-- Change job schedule
ALTER JOB daily_backup SCHEDULE 'AT 01:00';

-- Disable job
ALTER JOB daily_backup DISABLE;

-- Enable job
ALTER JOB daily_backup ENABLE;

-- Change job action
ALTER JOB daily_backup
    EXEC PROCEDURE new_backup_procedure();
```

#### Run Job Manually

```sql
-- Execute job immediately
EXEC run_job('daily_backup');

-- Or via API
curl -X POST http://localhost:8080/api/v1/admin/jobs/daily_backup/run \
  -H "Authorization: Bearer ${ADMIN_TOKEN}"
```

#### Drop Job

```sql
-- Drop job
DROP JOB daily_backup;

-- Drop job and remove history
DROP JOB daily_backup CASCADE;
```

### 9.2 Maintenance Windows

#### Define Maintenance Window

```sql
-- Create maintenance window (Monday-Friday 2-4 AM)
CREATE MAINTENANCE WINDOW weekday_window
    SCHEDULE 'FROM 02:00 TO 04:00 ON MON,TUE,WED,THU,FRI'
    PRIORITY HIGH;

-- Create weekend maintenance window
CREATE MAINTENANCE WINDOW weekend_window
    SCHEDULE 'FROM 01:00 TO 06:00 ON SAT,SUN'
    PRIORITY NORMAL;
```

#### Assign Jobs to Window

```sql
-- Assign job to maintenance window
ALTER JOB weekly_maintenance
    MAINTENANCE WINDOW weekend_window;

-- View jobs by maintenance window
SELECT
    w.window_name,
    w.schedule,
    j.job_name,
    j.last_run,
    j.next_run
FROM dba_maintenance_windows w
JOIN dba_jobs j ON j.maintenance_window = w.window_name
ORDER BY w.window_name, j.job_name;
```

#### Monitor Maintenance Windows

```sql
-- View upcoming maintenance windows
SELECT
    window_name,
    next_start,
    next_end,
    EXTRACT(EPOCH FROM (next_end - next_start)) / 60 as duration_minutes,
    job_count
FROM v$maintenance_windows_upcoming
ORDER BY next_start;
```

### 9.3 Common Scheduled Tasks

#### Backup Job

```sql
CREATE JOB full_backup_daily
    SCHEDULE 'AT 02:00'
    EXEC SQL $$
        EXEC backup_database(
            backup_type => 'FULL',
            compression => true,
            destination => '/var/lib/rustydb/backup'
        );
    $$;

CREATE JOB incremental_backup_hourly
    SCHEDULE 'CRON 0 * * * *'
    EXEC SQL $$
        EXEC backup_database(
            backup_type => 'INCREMENTAL',
            compression => true
        );
    $$;
```

#### Statistics Collection Job

```sql
CREATE JOB collect_stats_daily
    SCHEDULE 'AT 04:00'
    EXEC SQL $$
        EXEC collect_statistics(
            scope => 'DATABASE',
            estimate_percent => 10
        );
    $$;
```

#### Archive Log Cleanup Job

```sql
CREATE JOB cleanup_archive_logs
    SCHEDULE 'AT 01:00'
    EXEC SQL $$
        EXEC cleanup_archived_logs(
            older_than_days => 7,
            must_be_backed_up => true
        );
    $$;
```

#### Performance Snapshot Job

```sql
CREATE JOB capture_perf_snapshot
    SCHEDULE 'CRON 0 * * * *'  -- Every hour
    EXEC SQL $$
        EXEC capture_snapshot();
    $$;
```

#### Health Check Job

```sql
CREATE JOB health_check_hourly
    SCHEDULE 'CRON 0 * * * *'
    EXEC SQL $$
        EXEC run_health_checks();
    $$;
```

### 9.4 Job Monitoring and Alerting

#### Monitor Job Failures

```sql
-- View recent job failures
SELECT
    job_name,
    start_time,
    error_message,
    error_code
FROM dba_job_history
WHERE status = 'FAILED'
  AND start_time > NOW() - INTERVAL '7 days'
ORDER BY start_time DESC;

-- Count failures by job
SELECT
    job_name,
    COUNT(*) as failure_count,
    MAX(start_time) as last_failure
FROM dba_job_history
WHERE status = 'FAILED'
  AND start_time > NOW() - INTERVAL '30 days'
GROUP BY job_name
HAVING COUNT(*) > 3
ORDER BY failure_count DESC;
```

#### Set Up Job Alerts

```sql
-- Create alert for job failures
EXEC configure_alert(
    name => 'job_failure_alert',
    metric => 'job_status',
    condition => 'FAILED',
    severity => 'ERROR',
    action => 'NOTIFY_DBA'
);

-- Create alert for long-running jobs
EXEC configure_alert(
    name => 'long_running_job',
    metric => 'job_duration_minutes',
    threshold => 60,
    severity => 'WARNING',
    action => 'NOTIFY_DBA'
);
```

#### Job Performance Analysis

```sql
-- Analyze job execution times
SELECT
    job_name,
    COUNT(*) as executions,
    ROUND(AVG(EXTRACT(EPOCH FROM (end_time - start_time)) / 60), 2) as avg_duration_min,
    ROUND(MAX(EXTRACT(EPOCH FROM (end_time - start_time)) / 60), 2) as max_duration_min,
    ROUND(MIN(EXTRACT(EPOCH FROM (end_time - start_time)) / 60), 2) as min_duration_min
FROM dba_job_history
WHERE status = 'SUCCESS'
  AND start_time > NOW() - INTERVAL '30 days'
GROUP BY job_name
ORDER BY avg_duration_min DESC;
```

---

## 10. Performance Management

### 10.1 Identifying Performance Issues

#### System Performance Overview

```sql
-- Overall system metrics
SELECT
    ROUND(queries_per_second, 2) as qps,
    ROUND(transactions_per_second, 2) as tps,
    ROUND(cpu_usage_percent, 2) as cpu_percent,
    ROUND(memory_usage_percent, 2) as memory_percent,
    ROUND(buffer_pool_hit_ratio * 100, 2) as cache_hit_percent,
    active_connections,
    blocked_sessions
FROM v$system_performance;
```

#### Top Wait Events

```sql
-- Identify performance bottlenecks
SELECT
    wait_class,
    wait_event,
    wait_count,
    ROUND(total_wait_time_ms / 1000.0, 2) as total_wait_sec,
    ROUND(avg_wait_time_ms, 2) as avg_wait_ms
FROM v$system_wait_events
ORDER BY total_wait_time_ms DESC
LIMIT 20;
```

#### Slow Query Analysis

```sql
-- Top slow queries
SELECT
    sql_id,
    LEFT(sql_text, 100) as query,
    executions,
    ROUND(avg_time_ms, 2) as avg_ms,
    ROUND(max_time_ms, 2) as max_ms,
    ROUND(total_time_ms / 1000.0, 2) as total_sec
FROM v$sql_statistics
WHERE avg_time_ms > 1000  -- Queries averaging > 1 second
ORDER BY total_time_ms DESC
LIMIT 20;
```

### 10.2 Query Optimization

#### Explain Query Plans

```sql
-- Explain query execution plan
EXPLAIN SELECT * FROM orders WHERE customer_id = 12345;

-- Explain with execution statistics
EXPLAIN ANALYZE SELECT ...;
```

#### Use Query Hints

```sql
-- Force index usage
SELECT /*+ INDEX(orders idx_customer_id) */
    *
FROM orders
WHERE customer_id = 12345;

-- Force parallel execution
SELECT /*+ PARALLEL(8) */
    SUM(amount)
FROM large_table;

-- Disable parallel execution
SELECT /*+ NO_PARALLEL */
    *
FROM small_table;
```

#### Create Missing Indexes

```sql
-- Find missing indexes
SELECT
    schema_name,
    table_name,
    recommended_index_columns,
    estimated_improvement_percent,
    query_count
FROM v$missing_indexes
WHERE estimated_improvement_percent > 20
ORDER BY estimated_improvement_percent DESC;

-- Create recommended index
CREATE INDEX idx_orders_customer_date
    ON orders (customer_id, order_date);
```

### 10.3 Resource Monitoring

See **OPERATIONS.md** for comprehensive resource monitoring procedures.

Quick reference:

```sql
-- CPU usage by query
SELECT query_id, sql_text, cpu_time_ms
FROM v$active_queries
ORDER BY cpu_time_ms DESC;

-- Memory usage by session
SELECT session_id, user_name, pga_allocated_mb
FROM v$session
ORDER BY pga_allocated_mb DESC;

-- I/O statistics
SELECT tablespace_name, reads, writes
FROM v$tablespace_io_stats
ORDER BY reads + writes DESC;
```

---

## 11. Backup and Recovery

### 11.1 Backup Strategy

See **DEPLOYMENT_GUIDE.md** Section 9 for comprehensive backup procedures.

#### Backup Types

1. **Full Backup** - Complete database backup
2. **Incremental Backup** - Changes since last backup
3. **WAL Archive** - Continuous archiving for PITR

#### Recommended Backup Schedule

```
Daily:       Full backup (2 AM)
Hourly:      Incremental backup
Continuous:  WAL archiving
Weekly:      Backup validation and test restore
Monthly:     Off-site backup replication
```

### 11.2 Quick Backup Procedures

#### Full Backup

```bash
# Full backup via API
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -d '{
    "type": "FULL",
    "compression": true,
    "destination": "/var/lib/rustydb/backup/full_$(date +%Y%m%d).tar.gz"
  }'

# Or via CLI
rusty-db-backup full --output /var/lib/rustydb/backup/full_$(date +%Y%m%d).tar.gz
```

#### Incremental Backup

```bash
rusty-db-backup incremental --output /var/lib/rustydb/backup/incr_$(date +%Y%m%d_%H%M).tar.gz
```

### 11.3 Recovery Procedures

#### Restore from Backup

```bash
# Stop database
sudo systemctl stop rustydb

# Restore data
rusty-db-backup restore --file /var/lib/rustydb/backup/full_20241225.tar.gz

# Start database
sudo systemctl start rustydb
```

#### Point-in-Time Recovery (PITR)

```bash
# Restore to specific timestamp
rusty-db-backup restore \
  --file /var/lib/rustydb/backup/full_20241225.tar.gz \
  --point-in-time "2024-12-25 15:30:00"
```

---

## 12. Security Administration

### 12.1 Security Best Practices

See **DEPLOYMENT_GUIDE.md** Section 6 for comprehensive security hardening.

#### Key Security Tasks

1. **User Management**
   - Use strong passwords
   - Implement password policies
   - Regular user audits
   - Remove inactive users

2. **Privilege Management**
   - Follow least privilege principle
   - Use roles instead of direct grants
   - Regular privilege audits

3. **Audit Logging**
   - Enable comprehensive auditing
   - Regular audit log review
   - Archive audit logs

4. **Encryption**
   - Enable TDE (Transparent Data Encryption)
   - Use TLS for network connections
   - Encrypt backups

### 12.2 Security Monitoring

```sql
-- Failed login attempts
SELECT
    username,
    client_address,
    COUNT(*) as failed_attempts,
    MAX(attempt_time) as last_attempt
FROM v$login_failures
WHERE attempt_time > NOW() - INTERVAL '24 hours'
GROUP BY username, client_address
HAVING COUNT(*) > 5
ORDER BY failed_attempts DESC;

-- Users with admin privileges
SELECT
    grantee,
    granted_role,
    admin_option,
    granted_by
FROM dba_role_privs
WHERE granted_role IN ('SYSDBA', 'SYSOPER')
ORDER BY grantee;

-- Review audit trail
SELECT
    event_type,
    username,
    object_name,
    event_time,
    sql_text
FROM v$audit_trail
WHERE event_time > NOW() - INTERVAL '24 hours'
  AND event_type IN ('CREATE', 'DROP', 'ALTER', 'GRANT', 'REVOKE')
ORDER BY event_time DESC;
```

---

## 13. Appendices

### Appendix A: System Views Reference

| View | Description |
|------|-------------|
| v$session | Active database sessions |
| v$sql | SQL execution statistics |
| v$tablespace | Tablespace information and usage |
| v$datafile | Datafile information |
| v$log | Redo log information |
| v$archived_log | Archive log status |
| v$parameter | Configuration parameters |
| v$buffer_pool_stats | Buffer pool statistics |
| v$system_wait_events | Wait event statistics |
| v$health_check | System health status |
| v$replication_status | Replication status (HA setups) |
| v$backup_history | Backup history |
| dba_users | All database users |
| dba_roles | All database roles |
| dba_objects | All database objects |
| dba_tables | All tables |
| dba_indexes | All indexes |
| dba_segments | All segments (storage) |
| dba_jobs | Scheduled jobs |

### Appendix B: Configuration Quick Reference

**Essential Configuration Parameters:**

```toml
[server]
listen_port = 5432
max_connections = 1000
query_timeout_ms = 600000

[storage]
page_size = 8192                 # 8 KB pages (verified default)

[memory]
buffer_pool_size = 8192000       # ~8 MB (1000 pages × 8192 bytes, default)
shared_memory_size = 2147483648  # 2 GB

[wal]
enabled = true
checkpoint_interval_ms = 300000  # 5 minutes (300 seconds, verified)
archive_enabled = true

[monitoring]
prometheus_port = 9090           # Metrics port (verified)

[logging]
slow_query_threshold_ms = 1000   # 1 second (verified)

[performance]
max_parallel_workers = 16        # Worker threads (verified)
simd_enabled = true
io_uring_enabled = true
```

### Appendix C: Common DBA Commands

```sql
-- Server management
STARTUP;
SHUTDOWN;
SHUTDOWN IMMEDIATE;

-- Session management
ALTER SYSTEM KILL SESSION 'session_id';

-- Maintenance
VACUUM;
VACUUM FULL table_name;
ANALYZE;
REINDEX;

-- Space management
ALTER TABLESPACE name ADD DATAFILE 'path' SIZE 1G;
ALTER DATABASE DATAFILE 'path' RESIZE 2G;

-- Statistics
EXEC collect_statistics();
EXEC capture_snapshot();

-- Backup
EXEC backup_database();
```

### Appendix D: Performance Tuning Checklist

- [ ] Buffer pool hit ratio > 95%
- [ ] No tables with >20% bloat
- [ ] No unused indexes
- [ ] Statistics up to date (< 7 days)
- [ ] No queries averaging > 1 second (OLTP)
- [ ] No blocking sessions
- [ ] Checkpoint completing within interval
- [ ] WAL archiving not falling behind
- [ ] Disk I/O < 80% capacity
- [ ] Connection pool < 80% utilized

### Appendix E: Emergency Procedures

#### Database Won't Start

```bash
# Check logs
sudo journalctl -u rustydb -n 100

# Try recovery mode
sudo -u rustydb rusty-db-server --config /etc/rustydb/config.toml --recovery

# If corrupted, restore from backup
rusty-db-backup restore --file /path/to/latest/backup.tar.gz
```

#### Database Hung/Unresponsive

```bash
# Check system resources
top
iostat -x 1 10
free -h

# Kill blocking sessions
rusty-db-cli -c "ALTER SYSTEM KILL SESSION 'blocking_session_id';"

# Force checkpoint
rusty-db-cli -c "ALTER SYSTEM CHECKPOINT;"

# Last resort: immediate shutdown
rusty-db-cli -c "SHUTDOWN IMMEDIATE;"
```

#### Disk Full

```bash
# Immediate actions:
# 1. Delete old archive logs
rm /var/lib/rustydb/backup/archive/*.wal.old

# 2. Truncate old log files
> /var/log/rustydb/rustydb.log.old

# 3. Clean temp files
rm -rf /var/lib/rustydb/tmp/*

# 4. Add disk space or extend tablespace
```

### Appendix F: Useful Scripts

All maintenance scripts referenced in this guide:

- `/usr/local/bin/rustydb-healthcheck.sh` - Daily health check
- `/usr/local/bin/rustydb-daily-maintenance.sh` - Daily maintenance
- `/usr/local/bin/rustydb-weekly-maintenance.sh` - Weekly maintenance
- `/usr/local/bin/rustydb-monthly-maintenance.sh` - Monthly maintenance
- `/usr/local/bin/rustydb-archive-cleanup.sh` - Archive log cleanup
- `/usr/local/bin/rustydb-capacity-forecast.sh` - Capacity planning

### Appendix G: Support Contacts

**Internal DBA Team:**
- Email: dba-team@company.com
- On-call: dba-oncall@company.com
- Slack: #database-team

**RustyDB Enterprise Support:**
- Email: support@rustydb.io
- Portal: https://support.rustydb.io
- Emergency: +1-555-RUSTYDB

**Documentation:**
- GitHub: https://github.com/harborgrid-justin/rusty-db
- Docs: /home/user/rusty-db/docs/

---

## Document Control

**Version:** 1.0
**Status:** Production
**Classification:** Internal Use
**Last Review:** December 2024
**Next Review:** March 2025
**Owner:** Database Administration Team

**Change History:**

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2024-12-25 | DBA Team | Initial release for v0.5.1 |

---

**End of Administration Guide**

For additional information, refer to:
- **OPERATIONS.md** - Advanced operations and monitoring
- **DEPLOYMENT_GUIDE.md** - Installation and deployment
- **SECURITY_ARCHITECTURE.md** - Security configuration
- **CLAUDE.md** - Architecture and development reference
