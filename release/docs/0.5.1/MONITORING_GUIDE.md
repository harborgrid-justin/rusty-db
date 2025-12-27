# RustyDB Monitoring Guide

**Version**: 0.5.1
**Document Type**: Enterprise Monitoring Guide
**Last Updated**: 2025-12-27
**Audience**: Database Administrators, DevOps Engineers, Operations Teams

---

## Table of Contents

1. [Monitoring Overview](#monitoring-overview)
2. [Metrics Collection](#metrics-collection)
3. [Performance Metrics](#performance-metrics)
4. [Health Monitoring](#health-monitoring)
5. [Log Management](#log-management)
6. [Dashboard Integration](#dashboard-integration)
7. [Alerting](#alerting)
8. [Diagnostics](#diagnostics)
9. [Active Session History (ASH)](#active-session-history-ash)
10. [Automatic Workload Repository (AWR)](#automatic-workload-repository-awr)
11. [Best Practices](#best-practices)
12. [Troubleshooting](#troubleshooting)

---

## Monitoring Overview

RustyDB provides enterprise-grade monitoring capabilities inspired by Oracle Database's monitoring architecture, including Active Session History (ASH), automatic workload repository, and comprehensive metrics collection.

### Monitoring Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    RustyDB Monitoring Hub                    │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐     │
│  │   Metrics   │  │    Query     │  │   Active       │     │
│  │  Registry   │  │   Profiler   │  │   Session      │     │
│  │ (Prometheus)│  │              │  │   History      │     │
│  └─────────────┘  └──────────────┘  └────────────────┘     │
│                                                               │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐     │
│  │   Alert     │  │  Resource    │  │  Diagnostics   │     │
│  │  Manager    │  │  Manager     │  │  Repository    │     │
│  └─────────────┘  └──────────────┘  └────────────────┘     │
│                                                               │
│  ┌─────────────┐  ┌──────────────┐                          │
│  │  Dashboard  │  │ Statistics   │                          │
│  │ Aggregator  │  │  Collector   │                          │
│  └─────────────┘  └──────────────┘                          │
│                                                               │
└─────────────────────────────────────────────────────────────┘
         │                  │                    │
         ▼                  ▼                    ▼
   Prometheus         REST API             WebSocket
   (Port 9187)      (/api/v1/metrics)    (Real-time)
```

### Key Monitoring Components

1. **Metrics Registry**: Prometheus-compatible metrics collection with counters, gauges, histograms, and summaries
2. **Query Profiler**: Detailed query execution profiling with plan analysis
3. **Active Session History (ASH)**: Oracle-inspired session sampling for historical analysis
4. **Alert Manager**: Threshold-based and anomaly detection alerting
5. **Diagnostics**: Automatic incident detection and diagnostic dumps
6. **Dashboard**: Real-time metrics aggregation and visualization
7. **Statistics Collector**: Oracle-style V$ views for system statistics

### Monitoring Categories

| Category | Metrics | Purpose |
|----------|---------|---------|
| **Performance** | Query latency, TPS, QPS, cache hit ratio | Track database performance |
| **Resource** | CPU, memory, disk I/O, network | Monitor resource utilization |
| **Availability** | Uptime, connection success rate, replication lag | Ensure high availability |
| **Capacity** | Disk usage, buffer pool, connection pool | Plan for capacity |
| **Security** | Failed logins, audit events, privilege violations | Security monitoring |
| **Data Integrity** | Checksum failures, corruption events | Data quality |

---

## Metrics Collection

### Built-in Metrics

RustyDB automatically collects the following metrics:

#### Database Metrics

```rust
// Core database metrics
rustydb_uptime_seconds              // Database uptime
rustydb_queries_total                // Total queries executed
rustydb_queries_errors               // Total query errors
rustydb_active_connections           // Current active connections
rustydb_active_transactions          // Current active transactions
rustydb_transactions_total{status}   // Transactions (committed/rolled_back)
```

#### Performance Metrics

```rust
// Query performance
rustydb_query_duration_ms            // Query execution duration (histogram)
rustydb_queries_per_second           // Queries per second (gauge)
rustydb_transactions_per_second      // Transactions per second (gauge)

// Buffer pool metrics
rustydb_buffer_pool_size_bytes       // Buffer pool size
rustydb_buffer_pool_used_bytes       // Buffer pool used
rustydb_buffer_pool_hit_ratio        // Cache hit ratio (0.0-1.0)
rustydb_buffer_pool_hits             // Cache hits (counter)
rustydb_buffer_pool_misses           // Cache misses (counter)
```

#### Storage Metrics

```rust
// Disk I/O
rustydb_disk_reads_total             // Total disk reads
rustydb_disk_writes_total            // Total disk writes
rustydb_disk_reads_per_second        // Disk reads/second
rustydb_disk_writes_per_second       // Disk writes/second
rustydb_disk_read_latency_ms         // Disk read latency (histogram)
rustydb_disk_write_latency_ms        // Disk write latency (histogram)
```

#### Connection Metrics

```rust
// Connection pool
rustydb_connections_total            // Total connections
rustydb_connections_active           // Active connections
rustydb_connections_idle             // Idle connections
rustydb_connection_wait_queue_depth  // Waiting for connection
rustydb_connection_timeouts_total    // Connection timeouts
```

### Prometheus Integration

#### Configure Prometheus Exporter

RustyDB exposes metrics in Prometheus text format on a dedicated endpoint.

**Installation**:

```bash
# Metrics are exposed on the API server
# Default endpoint: http://localhost:8080/api/v1/metrics/prometheus

# Or use dedicated exporter (if configured)
wget https://github.com/rustydb/rustydb_exporter/releases/download/v1.0.0/rustydb_exporter
chmod +x rustydb_exporter
sudo mv rustydb_exporter /usr/local/bin/
```

**Create systemd service**:

```bash
sudo tee /etc/systemd/system/rustydb-exporter.service > /dev/null <<EOF
[Unit]
Description=RustyDB Prometheus Exporter
After=network.target rusty-db.service

[Service]
Type=simple
User=rustydb
ExecStart=/usr/local/bin/rustydb_exporter \\
  --database.dsn="postgresql://monitor:password@localhost:5432/rustydb" \\
  --web.listen-address=:9187 \\
  --log.level=info
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable rustydb-exporter
sudo systemctl start rustydb-exporter

# Verify
curl http://localhost:9187/metrics
```

#### Configure Prometheus

Add RustyDB to Prometheus configuration:

```yaml
# /etc/prometheus/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'rustydb'
    static_configs:
      - targets: ['localhost:9187']
        labels:
          environment: 'production'
          cluster: 'main'

    # Optional: Use API endpoint directly
  - job_name: 'rustydb-api'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/api/v1/metrics/prometheus'

  # Multi-instance setup
  - job_name: 'rustydb-cluster'
    static_configs:
      - targets:
        - 'rustydb-node1:9187'
        - 'rustydb-node2:9187'
        - 'rustydb-node3:9187'
        labels:
          cluster: 'main'
```

**Restart Prometheus**:

```bash
sudo systemctl restart prometheus

# Verify targets
curl http://localhost:9090/api/v1/targets
```

### Custom Metrics

#### Via REST API

Register custom metrics through the API:

```bash
# Register a counter
curl -X POST http://localhost:8080/api/v1/admin/metrics/register \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "custom_events",
    "type": "counter",
    "help": "Custom application events",
    "labels": {"app": "myapp", "env": "prod"}
  }'

# Increment counter
curl -X POST http://localhost:8080/api/v1/admin/metrics/custom_events/inc \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"value": 1}'

# Register a gauge
curl -X POST http://localhost:8080/api/v1/admin/metrics/register \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "queue_depth",
    "type": "gauge",
    "help": "Current queue depth"
  }'

# Set gauge value
curl -X POST http://localhost:8080/api/v1/admin/metrics/queue_depth/set \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"value": 42}'
```

#### Via SQL

```sql
-- Record custom metric (extension function)
SELECT rustydb_record_metric('custom_metric', 'counter', 1.0);
SELECT rustydb_set_metric('custom_gauge', 'gauge', 42.0);
```

### Metrics Endpoint Format

#### Prometheus Text Format

```bash
curl http://localhost:9187/metrics
```

**Example output**:

```
# HELP rustydb_uptime_seconds Database uptime in seconds
# TYPE rustydb_uptime_seconds counter
rustydb_uptime_seconds 86400

# HELP rustydb_active_connections Number of active connections
# TYPE rustydb_active_connections gauge
rustydb_active_connections 45

# HELP rustydb_queries_total Total number of queries executed
# TYPE rustydb_queries_total counter
rustydb_queries_total 1000000

# HELP rustydb_query_duration_ms Query execution duration in milliseconds
# TYPE rustydb_query_duration_ms histogram
rustydb_query_duration_ms_bucket{le="1"} 50000
rustydb_query_duration_ms_bucket{le="5"} 80000
rustydb_query_duration_ms_bucket{le="10"} 90000
rustydb_query_duration_ms_bucket{le="50"} 95000
rustydb_query_duration_ms_bucket{le="100"} 98000
rustydb_query_duration_ms_bucket{le="500"} 99500
rustydb_query_duration_ms_bucket{le="1000"} 99900
rustydb_query_duration_ms_bucket{le="+Inf"} 100000
rustydb_query_duration_ms_sum 234567.89
rustydb_query_duration_ms_count 100000

# HELP rustydb_buffer_pool_hit_ratio Buffer pool hit ratio
# TYPE rustydb_buffer_pool_hit_ratio gauge
rustydb_buffer_pool_hit_ratio 0.982
```

#### JSON Format

```bash
curl http://localhost:8080/api/v1/metrics
```

**Example output**:

```json
{
  "success": true,
  "data": {
    "database": {
      "uptime_seconds": 86400,
      "active_connections": 45,
      "transactions_per_second": 1234.56,
      "queries_per_second": 5678.90
    },
    "storage": {
      "buffer_pool_size_bytes": 1073741824,
      "buffer_pool_used_bytes": 858993459,
      "buffer_pool_hit_ratio": 0.982,
      "disk_reads_per_second": 123.45,
      "disk_writes_per_second": 67.89
    },
    "transactions": {
      "active_transactions": 12,
      "committed_total": 1000000,
      "rolled_back_total": 1234
    },
    "query_performance": {
      "avg_query_time_ms": 45.67,
      "p95_query_time_ms": 123.45,
      "p99_query_time_ms": 234.56
    }
  }
}
```

---

## Performance Metrics

### Query Performance Monitoring

#### Slow Query Detection

```bash
# Get slow queries (execution time > threshold)
curl http://localhost:8080/api/v1/stats/queries?limit=20 \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "slow_queries": [
      {
        "query_id": "query_12345",
        "sql": "SELECT * FROM large_table WHERE complex_condition",
        "avg_time_ms": 5678.90,
        "calls": 1000,
        "total_time_ms": 5678900,
        "p95_time_ms": 8900,
        "p99_time_ms": 12000,
        "rows_returned_avg": 50000
      }
    ],
    "top_queries_by_calls": [
      {
        "query_id": "query_67890",
        "sql": "SELECT id, name FROM users WHERE id = $1",
        "calls": 1000000,
        "avg_time_ms": 1.23,
        "total_time_ms": 1230000
      }
    ]
  }
}
```

#### Query Profiling

```bash
# Get detailed query profile
curl http://localhost:8080/api/v1/profiler/query/12345 \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "query_id": 12345,
    "sql": "SELECT * FROM orders JOIN customers ON orders.customer_id = customers.id",
    "execution_plan": {
      "operator": "HashJoin",
      "estimated_cost": 1234.56,
      "actual_cost": 1456.78,
      "children": [
        {
          "operator": "SeqScan",
          "table": "orders",
          "rows_returned": 10000,
          "execution_time_ms": 234
        },
        {
          "operator": "IndexScan",
          "table": "customers",
          "index": "customers_pkey",
          "rows_returned": 5000,
          "execution_time_ms": 123
        }
      ]
    },
    "execution_stats": {
      "total_execution_time_ms": 456.78,
      "cpu_time_ms": 234.56,
      "io_time_ms": 222.22,
      "rows_returned": 10000,
      "bytes_read": 1048576,
      "cache_hits": 9500,
      "cache_misses": 500
    },
    "wait_events": [
      {
        "event": "IO_READ",
        "wait_time_ms": 150,
        "wait_count": 50
      }
    ]
  }
}
```

### Transaction Metrics

```bash
# Get transaction statistics
curl http://localhost:8080/api/v1/stats/transactions \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "active_transactions": 12,
    "total_transactions": 1000000,
    "committed": 998766,
    "rolled_back": 1234,
    "avg_transaction_time_ms": 45.67,
    "deadlocks_total": 5,
    "long_running_transactions": [
      {
        "transaction_id": "txn_12345",
        "start_time": "2025-12-27T10:00:00Z",
        "duration_seconds": 300,
        "isolation_level": "SERIALIZABLE",
        "locks_held": 25
      }
    ]
  }
}
```

### Buffer Pool Performance

```bash
# Get buffer pool statistics
curl http://localhost:8080/api/v1/stats/buffer-pool \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "size_bytes": 1073741824,
    "used_bytes": 858993459,
    "free_bytes": 214748365,
    "usage_percent": 80.0,
    "hit_ratio": 0.982,
    "page_reads": 1000000,
    "page_writes": 500000,
    "evictions": 10000,
    "dirty_pages": 5000,
    "eviction_policy": "CLOCK",
    "pages_by_state": {
      "clean": 150000,
      "dirty": 5000,
      "pinned": 1000
    }
  }
}
```

### I/O Performance

```bash
# Get I/O statistics
curl http://localhost:8080/api/v1/stats/io \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "disk_reads_per_second": 123.45,
    "disk_writes_per_second": 67.89,
    "avg_read_latency_ms": 5.67,
    "avg_write_latency_ms": 8.90,
    "p95_read_latency_ms": 12.34,
    "p95_write_latency_ms": 15.67,
    "total_reads": 10000000,
    "total_writes": 5000000,
    "bytes_read": 41943040000,
    "bytes_written": 20971520000,
    "io_by_file_type": {
      "data_files": {
        "reads": 8000000,
        "writes": 3000000
      },
      "wal_files": {
        "reads": 1000000,
        "writes": 2000000
      },
      "temp_files": {
        "reads": 1000000,
        "writes": 0
      }
    }
  }
}
```

---

## Health Monitoring

### Health Check Endpoints

#### Overall Health

```bash
curl http://localhost:8080/api/v1/admin/health
```

**Response**:

```json
{
  "success": true,
  "data": {
    "status": "HEALTHY",
    "timestamp": "2025-12-27T10:00:00Z",
    "components": {
      "database": {
        "status": "UP",
        "uptime_sec": 86400,
        "version": "0.5.1"
      },
      "storage": {
        "status": "UP",
        "disk_usage_percent": 45.6,
        "buffer_pool_hit_ratio": 0.982
      },
      "connections": {
        "status": "UP",
        "active": 45,
        "max": 100,
        "utilization_percent": 45
      },
      "replication": {
        "status": "UP",
        "lag_bytes": 1024,
        "replicas_healthy": 2,
        "replicas_total": 2
      },
      "transactions": {
        "status": "UP",
        "active": 12,
        "deadlocks_last_hour": 0
      }
    }
  }
}
```

#### Liveness Probe

Lightweight check for Kubernetes/container orchestration:

```bash
curl http://localhost:8080/api/v1/health/live
```

**Response** (200 OK if alive):

```json
{
  "status": "alive",
  "timestamp": "2025-12-27T10:00:00Z"
}
```

#### Readiness Probe

Check if ready to accept traffic:

```bash
curl http://localhost:8080/api/v1/health/ready
```

**Response** (200 OK if ready, 503 if not ready):

```json
{
  "status": "ready",
  "checks": {
    "database": true,
    "replication": true,
    "connections_available": true
  },
  "timestamp": "2025-12-27T10:00:00Z"
}
```

### Component Health Checks

RustyDB includes built-in health checks for critical components:

| Check | Criteria | Severity |
|-------|----------|----------|
| **Connection Pool** | < 95% utilization | Warning at 80%, Critical at 95% |
| **Memory** | Available memory | Warning at 80%, Critical at 90% |
| **Disk Space** | Free disk space | Warning at 75%, Critical at 85% |
| **Replication Lag** | Lag in seconds | Warning at 10s, Critical at 30s |
| **Buffer Pool Hit Ratio** | Cache efficiency | Warning at < 90%, Critical at < 80% |
| **Lock Contention** | Wait time | Warning at > 100ms avg, Critical at > 500ms |

### Health Monitoring Configuration

```yaml
# /etc/rusty-db/rusty-db.conf
[health_checks]
enabled = true
interval_seconds = 30

[health_checks.connection_pool]
warning_threshold_percent = 80
critical_threshold_percent = 95

[health_checks.memory]
warning_threshold_percent = 80
critical_threshold_percent = 90

[health_checks.disk_space]
warning_threshold_percent = 75
critical_threshold_percent = 85

[health_checks.replication_lag]
warning_threshold_seconds = 10
critical_threshold_seconds = 30

[health_checks.buffer_pool]
warning_hit_ratio = 0.90
critical_hit_ratio = 0.80
```

### Automated Health Check Script

```bash
#!/bin/bash
# /usr/local/bin/rustydb-health-monitor.sh

LOG_FILE="/var/log/rusty-db/health-monitor.log"
ALERT_EMAIL="dba-team@company.com"

# Check database health
HEALTH=$(curl -s http://localhost:8080/api/v1/admin/health)
STATUS=$(echo "$HEALTH" | jq -r '.data.status')

# Log health check
echo "$(date): Health status: $STATUS" >> $LOG_FILE

# Alert on unhealthy status
if [ "$STATUS" != "HEALTHY" ]; then
    echo "WARNING: Database health is $STATUS" | mail -s "RustyDB Health Alert" $ALERT_EMAIL

    # Get detailed component status
    echo "$HEALTH" | jq '.data.components' >> $LOG_FILE
fi

# Check individual components
DISK_USAGE=$(echo "$HEALTH" | jq -r '.data.components.storage.disk_usage_percent')
if (( $(echo "$DISK_USAGE > 80" | bc -l) )); then
    echo "WARNING: High disk usage: ${DISK_USAGE}%" | mail -s "RustyDB Disk Alert" $ALERT_EMAIL
fi

# Check replication lag
REPL_LAG=$(echo "$HEALTH" | jq -r '.data.components.replication.lag_bytes // 0')
if [ "$REPL_LAG" -gt 10485760 ]; then  # 10MB
    echo "WARNING: High replication lag: $REPL_LAG bytes" | mail -s "RustyDB Replication Alert" $ALERT_EMAIL
fi
```

**Schedule health monitoring**:

```bash
chmod +x /usr/local/bin/rustydb-health-monitor.sh

# Add to crontab (every 5 minutes)
*/5 * * * * /usr/local/bin/rustydb-health-monitor.sh
```

---

## Log Management

### Log Levels

RustyDB supports the following log levels:

| Level | Purpose | Default Output |
|-------|---------|----------------|
| **ERROR** | Critical errors requiring immediate attention | Always logged |
| **WARN** | Warning conditions that should be investigated | Always logged |
| **INFO** | Informational messages about normal operations | Logged by default |
| **DEBUG** | Detailed debugging information | Disabled by default |
| **TRACE** | Very detailed tracing (performance impact) | Disabled by default |

### Log Configuration

```yaml
# /etc/rusty-db/rusty-db.conf
[logging]
# Global log level
level = "INFO"

# Log file location
log_directory = "/var/log/rusty-db"

# Log file rotation
max_file_size_mb = 100
max_files = 30
compress_rotated = true

# Component-specific log levels
[logging.components]
query_executor = "DEBUG"
transaction_manager = "INFO"
storage = "INFO"
replication = "DEBUG"
security = "WARN"

# Slow query logging
[logging.slow_queries]
enabled = true
threshold_ms = 1000
log_plan = true
log_parameters = false  # For security

# Audit logging
[logging.audit]
enabled = true
log_successful_logins = true
log_failed_logins = true
log_ddl = true
log_privilege_changes = true
```

### Log File Locations

```
/var/log/rusty-db/
├── rusty-db.log           # Main application log
├── slow-query.log         # Slow query log
├── audit.log              # Security audit log
├── replication.log        # Replication events
├── backup.log             # Backup/restore operations
├── error.log              # Error-only log
└── archive/               # Rotated logs
    ├── rusty-db-20251226.log.gz
    └── slow-query-20251226.log.gz
```

### Log Rotation

Configure logrotate:

```bash
sudo tee /etc/logrotate.d/rustydb > /dev/null <<'EOF'
/var/log/rusty-db/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0640 rustydb rustydb
    sharedscripts
    postrotate
        systemctl reload rusty-db > /dev/null 2>&1 || true
    endscript
}

/var/log/rusty-db/audit.log {
    daily
    rotate 90
    compress
    delaycompress
    missingok
    notifempty
    create 0600 rustydb rustydb
    # Don't reload for audit log
}
EOF

# Test configuration
sudo logrotate -d /etc/logrotate.d/rustydb

# Force rotation
sudo logrotate -f /etc/logrotate.d/rustydb
```

### Slow Query Logging

Enable and configure slow query logging:

```sql
-- Enable slow query logging
ALTER SYSTEM SET slow_query_log = ON;
ALTER SYSTEM SET slow_query_threshold_ms = 1000;
ALTER SYSTEM SET slow_query_log_plan = ON;

-- View slow query log settings
SELECT name, setting, description
FROM rustydb_settings
WHERE name LIKE 'slow_query%';
```

**Analyze slow query log**:

```bash
# Find slowest queries
grep "Query time:" /var/log/rusty-db/slow-query.log | \
  awk '{print $3}' | \
  sort -rn | \
  head -20

# Get query details
tail -100 /var/log/rusty-db/slow-query.log
```

**Example slow query log entry**:

```
2025-12-27 10:15:23.456 [SLOW QUERY]
Query time: 2345.67 ms
Lock time: 12.34 ms
Rows examined: 100000
Rows returned: 50
Query: SELECT * FROM large_table WHERE complex_condition = 'value'
Query Plan:
  SeqScan on large_table (cost=0.00..25000.00 rows=100000)
    Filter: (complex_condition = 'value')
Suggestion: Create index on large_table(complex_condition)
```

### Centralized Logging

#### ELK Stack Integration

```bash
# Install Filebeat
curl -L -O https://artifacts.elastic.co/downloads/beats/filebeat/filebeat-8.11.0-amd64.deb
sudo dpkg -i filebeat-8.11.0-amd64.deb

# Configure Filebeat
sudo tee /etc/filebeat/filebeat.yml > /dev/null <<EOF
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/rusty-db/*.log
  fields:
    application: rustydb
    environment: production
    cluster: main
  multiline:
    pattern: '^[0-9]{4}-[0-9]{2}-[0-9]{2}'
    negate: true
    match: after

processors:
  - add_host_metadata: ~
  - add_cloud_metadata: ~

output.elasticsearch:
  hosts: ["elasticsearch.company.com:9200"]
  index: "rustydb-logs-%{+yyyy.MM.dd}"

setup.kibana:
  host: "kibana.company.com:5601"

logging.level: info
EOF

# Start Filebeat
sudo systemctl enable filebeat
sudo systemctl start filebeat
```

#### Syslog Integration

```bash
# Configure rsyslog
sudo tee /etc/rsyslog.d/30-rustydb.conf > /dev/null <<EOF
# RustyDB logs to syslog
\$ModLoad imfile
\$InputFileName /var/log/rusty-db/rusty-db.log
\$InputFileTag rustydb:
\$InputFileStateFile stat-rustydb
\$InputFileSeverity info
\$InputFileFacility local0
\$InputRunFileMonitor

# Forward to central syslog server
*.* @@syslog.company.com:514
EOF

sudo systemctl restart rsyslog
```

---

## Dashboard Integration

### Grafana Dashboards

#### Install Grafana

```bash
# Add Grafana repository
sudo apt-get install -y software-properties-common
sudo add-apt-repository "deb https://packages.grafana.com/oss/deb stable main"
wget -q -O - https://packages.grafana.com/gpg.key | sudo apt-key add -

# Install Grafana
sudo apt-get update
sudo apt-get install grafana

# Enable and start
sudo systemctl enable grafana-server
sudo systemctl start grafana-server

# Access: http://localhost:3000 (admin/admin)
```

#### Configure Prometheus Data Source

1. Login to Grafana (http://localhost:3000)
2. Go to Configuration → Data Sources
3. Add Prometheus data source:
   - URL: `http://localhost:9090`
   - Access: Server (default)
   - Scrape interval: 15s

#### RustyDB Overview Dashboard

Create a comprehensive dashboard with the following panels:

**Panel 1: Database Overview**
```sql
# Queries per second
rate(rustydb_queries_total[1m])

# Transactions per second
rate(rustydb_transactions_total{status="committed"}[1m])

# Active connections
rustydb_active_connections

# Uptime
rustydb_uptime_seconds / 3600 / 24  # Convert to days
```

**Panel 2: Query Performance**
```sql
# Average query time
rustydb_query_duration_ms_sum / rustydb_query_duration_ms_count

# 95th percentile query time
histogram_quantile(0.95, rate(rustydb_query_duration_ms_bucket[5m]))

# 99th percentile query time
histogram_quantile(0.99, rate(rustydb_query_duration_ms_bucket[5m]))
```

**Panel 3: Resource Utilization**
```sql
# CPU usage (if exported)
rustydb_cpu_usage_percent

# Memory usage
(rustydb_buffer_pool_used_bytes / rustydb_buffer_pool_size_bytes) * 100

# Buffer pool hit ratio
rustydb_buffer_pool_hit_ratio * 100
```

**Panel 4: I/O Performance**
```sql
# Disk reads/sec
rate(rustydb_disk_reads_total[1m])

# Disk writes/sec
rate(rustydb_disk_writes_total[1m])

# Read latency p95
histogram_quantile(0.95, rate(rustydb_disk_read_latency_ms_bucket[5m]))

# Write latency p95
histogram_quantile(0.95, rate(rustydb_disk_write_latency_ms_bucket[5m]))
```

**Panel 5: Connection Pool**
```sql
# Active connections
rustydb_connections_active

# Idle connections
rustydb_connections_idle

# Wait queue depth
rustydb_connection_wait_queue_depth

# Connection timeouts
rate(rustydb_connection_timeouts_total[5m])
```

**Panel 6: Replication (if applicable)**
```sql
# Replication lag
rustydb_replication_lag_bytes

# Replication lag time
rustydb_replication_lag_seconds
```

#### Import Pre-built Dashboard

```bash
# Download RustyDB Grafana dashboard
wget https://github.com/rustydb/grafana-dashboards/rustydb-overview.json

# Or create dashboard JSON
cat > rustydb-dashboard.json <<'EOF'
{
  "dashboard": {
    "title": "RustyDB Overview",
    "panels": [
      {
        "title": "Queries per Second",
        "targets": [
          {
            "expr": "rate(rustydb_queries_total[1m])"
          }
        ],
        "type": "graph"
      }
    ]
  }
}
EOF

# Import via Grafana UI or API
curl -X POST http://localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d @rustydb-dashboard.json
```

### Real-time Dashboard API

RustyDB provides a real-time dashboard data aggregator accessible via REST and WebSocket.

#### REST API

```bash
# Get current dashboard snapshot
curl http://localhost:8080/api/v1/dashboard/snapshot \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "timestamp": "2025-12-27T10:00:00Z",
    "performance": {
      "queries_per_second": 5678.90,
      "transactions_per_second": 1234.56,
      "avg_query_time_ms": 45.67,
      "p95_query_time_ms": 123.45,
      "p99_query_time_ms": 234.56
    },
    "resources": {
      "cpu_usage_percent": 45.6,
      "memory_usage_percent": 67.8,
      "disk_usage_percent": 34.5,
      "buffer_pool_hit_ratio": 98.2
    },
    "connections": {
      "active": 45,
      "idle": 50,
      "max": 100,
      "wait_queue_depth": 2
    },
    "top_queries": [
      {
        "query_id": 12345,
        "sql": "SELECT * FROM...",
        "calls": 1000,
        "avg_time_ms": 567.89,
        "total_cpu_ms": 45678
      }
    ],
    "replication": {
      "replicas": [
        {
          "name": "replica-1",
          "lag_bytes": 1024,
          "lag_seconds": 0.5,
          "status": "healthy"
        }
      ]
    }
  }
}
```

#### WebSocket Streaming

```javascript
// Real-time dashboard updates via WebSocket
const ws = new WebSocket('ws://localhost:8080/api/v1/dashboard/stream');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'subscribe',
    channels: ['performance', 'resources', 'alerts']
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Dashboard update:', data);

  // Update UI with new data
  updatePerformanceMetrics(data.performance);
  updateResourceMetrics(data.resources);
  updateAlerts(data.alerts);
};
```

### Key Visualizations

1. **Time Series Graphs**
   - Queries/Transactions per second
   - Query latency (p50, p95, p99)
   - CPU and memory usage
   - Disk I/O rates

2. **Gauges**
   - Current active connections
   - Buffer pool usage
   - Cache hit ratio
   - Replication lag

3. **Tables**
   - Top queries by execution time
   - Top queries by call count
   - Active sessions
   - Recent slow queries

4. **Heatmaps**
   - Query latency distribution over time
   - Resource usage patterns
   - I/O patterns by hour

---

## Alerting

### Alert Manager

RustyDB includes a built-in alert manager with support for threshold-based and anomaly detection alerting.

### Alert Categories

| Category | Description | Examples |
|----------|-------------|----------|
| **Performance** | Query and transaction performance | Slow queries, high latency |
| **Availability** | System uptime and accessibility | Database down, connection failures |
| **Capacity** | Resource limits and capacity | Disk full, memory exhaustion |
| **Security** | Security events and violations | Failed logins, privilege escalation |
| **Data Integrity** | Data consistency issues | Corruption, checksum failures |
| **Replication** | Replication health | High lag, replica down |
| **Backup** | Backup and recovery | Backup failures, restore issues |

### Threshold-Based Alerts

Configure threshold rules via REST API:

```bash
# Create high CPU alert
curl -X POST http://localhost:8080/api/v1/alerts/rules/threshold \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "high_cpu",
    "metric_name": "cpu_usage_percent",
    "threshold": 80.0,
    "comparison": "GreaterThan",
    "duration_seconds": 300,
    "severity": "Warning",
    "category": "Performance",
    "cooldown_seconds": 600
  }'

# Create critical memory alert
curl -X POST http://localhost:8080/api/v1/alerts/rules/threshold \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "critical_memory",
    "metric_name": "memory_usage_percent",
    "threshold": 90.0,
    "comparison": "GreaterThan",
    "severity": "Critical",
    "category": "Capacity"
  }'

# Create low cache hit ratio alert
curl -X POST http://localhost:8080/api/v1/alerts/rules/threshold \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "low_cache_hit_ratio",
    "metric_name": "buffer_pool_hit_ratio",
    "threshold": 0.80,
    "comparison": "LessThan",
    "severity": "Warning",
    "category": "Performance"
  }'
```

### Anomaly Detection Alerts

Configure anomaly detection rules:

```bash
# Create query latency anomaly detection
curl -X POST http://localhost:8080/api/v1/alerts/rules/anomaly \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "query_latency_anomaly",
    "metric_name": "avg_query_time_ms",
    "algorithm": "StandardDeviation",
    "sensitivity": 3.0,
    "window_size": 100,
    "severity": "Warning",
    "category": "Performance"
  }'

# Create disk I/O anomaly detection
curl -X POST http://localhost:8080/api/v1/alerts/rules/anomaly \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "disk_io_anomaly",
    "metric_name": "disk_writes_per_second",
    "algorithm": "InterquartileRange",
    "sensitivity": 1.5,
    "severity": "Info",
    "category": "Performance"
  }'
```

**Anomaly Detection Algorithms**:

- **StandardDeviation**: Detects values beyond N standard deviations from mean
- **InterquartileRange**: Uses IQR method to detect outliers
- **MovingAverage**: Compares to moving average window
- **ExponentialSmoothing**: Uses exponential smoothing forecast

### Alert Thresholds

Recommended alert thresholds:

```yaml
alerts:
  # Performance alerts
  - name: high_query_latency
    metric: p95_query_time_ms
    warning: 100
    critical: 500

  - name: low_cache_hit_ratio
    metric: buffer_pool_hit_ratio
    warning: 0.90
    critical: 0.80

  # Resource alerts
  - name: high_cpu
    metric: cpu_usage_percent
    warning: 75
    critical: 90

  - name: high_memory
    metric: memory_usage_percent
    warning: 80
    critical: 90

  - name: disk_space
    metric: disk_usage_percent
    warning: 75
    critical: 85

  # Connection alerts
  - name: connection_pool_exhaustion
    metric: connection_pool_utilization_percent
    warning: 80
    critical: 95

  # Replication alerts
  - name: replication_lag
    metric: replication_lag_seconds
    warning: 10
    critical: 30
```

### Alert Routing

Configure alert routing to different channels:

```bash
# Configure email notifications
curl -X POST http://localhost:8080/api/v1/alerts/routes \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "critical_alerts_email",
    "filters": {
      "severity": ["Critical", "Error"]
    },
    "destination": {
      "type": "email",
      "addresses": ["dba-team@company.com", "oncall@company.com"]
    }
  }'

# Configure Slack notifications
curl -X POST http://localhost:8080/api/v1/alerts/routes \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "performance_alerts_slack",
    "filters": {
      "category": ["Performance"],
      "severity": ["Warning", "Error", "Critical"]
    },
    "destination": {
      "type": "slack",
      "webhook_url": "https://hooks.slack.com/services/YOUR/WEBHOOK/URL",
      "channel": "#database-alerts"
    }
  }'

# Configure PagerDuty for critical alerts
curl -X POST http://localhost:8080/api/v1/alerts/routes \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "critical_pagerduty",
    "filters": {
      "severity": ["Critical"]
    },
    "destination": {
      "type": "pagerduty",
      "integration_key": "YOUR_PAGERDUTY_KEY"
    }
  }'
```

### Managing Alerts

```bash
# Get active alerts
curl http://localhost:8080/api/v1/alerts?status=active \
  -H "Authorization: Bearer $TOKEN"

# Get alerts by severity
curl http://localhost:8080/api/v1/alerts?severity=Critical \
  -H "Authorization: Bearer $TOKEN"

# Acknowledge alert
curl -X POST http://localhost:8080/api/v1/alerts/12345/acknowledge \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"note": "Investigating high CPU usage"}'

# Resolve alert
curl -X POST http://localhost:8080/api/v1/alerts/12345/resolve \
  -H "Authorization: Bearer $TOKEN"

# Suppress alert (temporary)
curl -X POST http://localhost:8080/api/v1/alerts/12345/suppress \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"duration_minutes": 60}'
```

### Prometheus Alertmanager Integration

```yaml
# prometheus-alerts.yml
groups:
  - name: rustydb_alerts
    rules:
      - alert: HighCPUUsage
        expr: rustydb_cpu_usage_percent > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage on {{ $labels.instance }}"
          description: "CPU usage is {{ $value }}%"

      - alert: HighMemoryUsage
        expr: (rustydb_buffer_pool_used_bytes / rustydb_buffer_pool_size_bytes) * 100 > 90
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High memory usage on {{ $labels.instance }}"
          description: "Memory usage is {{ $value }}%"

      - alert: LowCacheHitRatio
        expr: rustydb_buffer_pool_hit_ratio < 0.80
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low cache hit ratio on {{ $labels.instance }}"
          description: "Cache hit ratio is {{ $value }}"

      - alert: ReplicationLag
        expr: rustydb_replication_lag_seconds > 30
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High replication lag on {{ $labels.standby }}"
          description: "Replication lag is {{ $value }} seconds"

      - alert: DatabaseDown
        expr: up{job="rustydb"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Database is down on {{ $labels.instance }}"
          description: "RustyDB instance is not responding"
```

---

## Diagnostics

### Diagnostic Repository

RustyDB automatically captures diagnostic information when incidents occur.

### Incident Types

| Type | Description | Auto-Capture |
|------|-------------|--------------|
| **Crash** | Database crash or panic | Yes |
| **Hang** | Unresponsive database | Manual |
| **DataCorruption** | Data integrity violation | Yes |
| **PerformanceDegradation** | Significant performance drop | Threshold |
| **MemoryLeak** | Memory growth pattern | Threshold |
| **Deadlock** | Transaction deadlock | Yes |
| **ConnectionFailure** | Connection pool exhaustion | Threshold |
| **DiskFull** | Disk space exhausted | Yes |
| **ReplicationFailure** | Replication stopped/failed | Yes |

### Diagnostic Dumps

#### Automatic Dumps

Configure automatic diagnostic dumps:

```yaml
# /etc/rusty-db/rusty-db.conf
[diagnostics]
enabled = true
adr_base = "/var/lib/rusty-db/diagnostics"
max_incidents = 10000
auto_dump_on_critical = true

[diagnostics.dump_types]
system_state = true
process_state = true
memory_dump = false  # Can be large
lock_dump = true
transaction_dump = true
buffer_cache = false
error_stack = true
```

#### Manual Diagnostic Dump

```bash
# Create system state dump
curl -X POST http://localhost:8080/api/v1/diagnostics/dump \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"dump_type": "SystemState"}'

# Create full diagnostic dump
curl -X POST http://localhost:8080/api/v1/diagnostics/dump \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"dump_type": "Full"}'

# Create dump for specific incident
curl -X POST http://localhost:8080/api/v1/diagnostics/dump \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"dump_type": "Full", "incident_id": 12345}'
```

**Dump Types**:

- **SystemState**: Overall system state, configuration, metrics
- **ProcessState**: Process information, threads, stack traces
- **MemoryDump**: Memory allocation snapshot
- **LockDump**: Current locks and waiters
- **TransactionDump**: Active transactions
- **BufferCache**: Buffer pool contents
- **ErrorStack**: Recent errors and warnings
- **Full**: All of the above

### Incident Management

```bash
# List incidents
curl http://localhost:8080/api/v1/diagnostics/incidents \
  -H "Authorization: Bearer $TOKEN"

# Get incident details
curl http://localhost:8080/api/v1/diagnostics/incidents/12345 \
  -H "Authorization: Bearer $TOKEN"

# Create manual incident
curl -X POST http://localhost:8080/api/v1/diagnostics/incidents \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "incident_type": "PerformanceDegradation",
    "severity": "High",
    "description": "Query performance degraded after deployment"
  }'

# Resolve incident
curl -X POST http://localhost:8080/api/v1/diagnostics/incidents/12345/resolve \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"resolution": "Added missing index on table"}'
```

### Diagnostic Data Collection Script

```bash
#!/bin/bash
# /usr/local/bin/collect-rustydb-diagnostics.sh

DIAG_DIR="/tmp/rustydb-diagnostics-$(date +%Y%m%d_%H%M%S)"
mkdir -p $DIAG_DIR

echo "Collecting RustyDB diagnostic data..."

# System information
echo "=== System Information ===" > $DIAG_DIR/system-info.txt
uname -a >> $DIAG_DIR/system-info.txt
cat /etc/os-release >> $DIAG_DIR/system-info.txt
free -h >> $DIAG_DIR/system-info.txt
df -h >> $DIAG_DIR/system-info.txt
top -bn1 >> $DIAG_DIR/top-snapshot.txt

# RustyDB configuration
cp /etc/rusty-db/rusty-db.conf $DIAG_DIR/

# Database health
curl -s http://localhost:8080/api/v1/admin/health \
  -H "Authorization: Bearer $TOKEN" > $DIAG_DIR/health.json

# Database metrics
curl -s http://localhost:8080/api/v1/metrics \
  -H "Authorization: Bearer $TOKEN" > $DIAG_DIR/metrics.json

# Active sessions
curl -s http://localhost:8080/api/v1/sessions \
  -H "Authorization: Bearer $TOKEN" > $DIAG_DIR/sessions.json

# Recent logs
tail -10000 /var/log/rusty-db/rusty-db.log > $DIAG_DIR/rusty-db.log
tail -1000 /var/log/rusty-db/slow-query.log > $DIAG_DIR/slow-query.log
tail -1000 /var/log/rusty-db/error.log > $DIAG_DIR/error.log

# Create diagnostic dump via API
curl -X POST http://localhost:8080/api/v1/diagnostics/dump \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"dump_type": "Full"}' > $DIAG_DIR/full-dump.json

# Create tarball
tar -czf ${DIAG_DIR}.tar.gz -C /tmp $(basename $DIAG_DIR)
rm -rf $DIAG_DIR

echo "Diagnostics collected: ${DIAG_DIR}.tar.gz"
```

---

## Active Session History (ASH)

### Overview

Active Session History (ASH) is an Oracle-inspired feature that periodically samples active sessions to provide historical performance analysis.

### ASH Architecture

```
┌────────────────────────────────────────────┐
│     Active Session History (ASH)           │
├────────────────────────────────────────────┤
│                                            │
│  Sampling Thread (every 1 second)         │
│          │                                 │
│          ├─> Sample Active Sessions       │
│          ├─> Capture SQL, Wait Events     │
│          ├─> Record Resource Usage        │
│          └─> Store in Ring Buffer         │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │   ASH Sample Ring Buffer           │   │
│  │   (86,400 samples = 24 hours)      │   │
│  └────────────────────────────────────┘   │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │   SQL Statistics Aggregation       │   │
│  │   - Execution counts               │   │
│  │   - CPU/DB time                    │   │
│  │   - Wait event breakdown           │   │
│  └────────────────────────────────────┘   │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │   Session Statistics               │   │
│  │   - State breakdown                │   │
│  │   - Wait time distribution         │   │
│  └────────────────────────────────────┘   │
│                                            │
└────────────────────────────────────────────┘
```

### ASH Configuration

```yaml
# /etc/rusty-db/rusty-db.conf
[ash]
enabled = true
sample_interval_seconds = 1
max_samples = 86400  # 24 hours of history
retention_hours = 24

# What to capture
capture_sql_text = true
capture_wait_events = true
capture_blocking_sessions = true
capture_resource_usage = true
```

### ASH Sample Data

Each ASH sample contains:

- **Session Information**: Session ID, user ID, program name
- **Session State**: ACTIVE, IDLE, WAITING, BLOCKED
- **SQL Information**: SQL ID, SQL text, execution plan hash
- **Wait Events**: Wait class, wait event, wait time
- **Blocking**: Blocking session (if blocked)
- **Resource Usage**: CPU time, DB time, temp space, PGA
- **Object Access**: Current object, file, and block

### Querying ASH Data

```bash
# Get recent ASH samples
curl http://localhost:8080/api/v1/ash/samples?limit=100 \
  -H "Authorization: Bearer $TOKEN"

# Get ASH samples for specific session
curl http://localhost:8080/api/v1/ash/samples?session_id=12345 \
  -H "Authorization: Bearer $TOKEN"

# Get ASH samples for time range
curl "http://localhost:8080/api/v1/ash/samples?start=2025-12-27T10:00:00Z&end=2025-12-27T11:00:00Z" \
  -H "Authorization: Bearer $TOKEN"

# Get SQL statistics from ASH
curl http://localhost:8080/api/v1/ash/sql-stats?limit=20 \
  -H "Authorization: Bearer $TOKEN"

# Get session statistics from ASH
curl http://localhost:8080/api/v1/ash/session-stats?session_id=12345 \
  -H "Authorization: Bearer $TOKEN"
```

### ASH Analysis Examples

#### Top SQL by CPU Time

```bash
curl "http://localhost:8080/api/v1/ash/report?report_type=top_sql_by_cpu&limit=10" \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "report_type": "top_sql_by_cpu",
    "time_range": {
      "start": "2025-12-27T10:00:00Z",
      "end": "2025-12-27T11:00:00Z"
    },
    "top_sql": [
      {
        "sql_id": 12345,
        "sql_text": "SELECT * FROM large_table WHERE...",
        "executions": 1000,
        "total_cpu_time_ms": 567890,
        "avg_cpu_time_ms": 567.89,
        "total_samples": 5678,
        "percent_of_total_cpu": 45.6
      }
    ]
  }
}
```

#### Wait Event Analysis

```bash
curl "http://localhost:8080/api/v1/ash/report?report_type=wait_events" \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "report_type": "wait_events",
    "wait_classes": [
      {
        "wait_class": "UserIO",
        "total_wait_time_ms": 123456,
        "wait_count": 5000,
        "avg_wait_ms": 24.69,
        "percent_of_total": 60.5,
        "top_events": [
          {
            "event": "db_file_sequential_read",
            "wait_time_ms": 89012,
            "wait_count": 3000
          }
        ]
      },
      {
        "wait_class": "Concurrency",
        "total_wait_time_ms": 45678,
        "wait_count": 2000,
        "avg_wait_ms": 22.84,
        "percent_of_total": 22.4
      }
    ]
  }
}
```

#### Session Activity Report

```bash
curl "http://localhost:8080/api/v1/ash/report?report_type=session_activity&session_id=12345" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Automatic Workload Repository (AWR)

### AWR Overview

The Automatic Workload Repository (AWR) provides historical performance data through periodic snapshots, similar to Oracle Database AWR.

### AWR Architecture

```
┌─────────────────────────────────────────────┐
│   Automatic Workload Repository (AWR)       │
├─────────────────────────────────────────────┤
│                                             │
│  Snapshot Schedule (hourly)                │
│          │                                  │
│          ├─> Capture System Metrics        │
│          ├─> Aggregate ASH Samples         │
│          ├─> Capture SQL Statistics        │
│          ├─> Capture Wait Events           │
│          ├─> Capture I/O Statistics        │
│          └─> Store Snapshot                │
│                                             │
│  ┌───────────────────────────────────┐     │
│  │   AWR Snapshot Storage            │     │
│  │   (Retention: 30 days)            │     │
│  └───────────────────────────────────┘     │
│                                             │
│  ┌───────────────────────────────────┐     │
│  │   AWR Report Generator            │     │
│  │   - Performance summary            │     │
│  │   - Top SQL                        │     │
│  │   - Wait events                    │     │
│  │   - I/O statistics                 │     │
│  │   - Trend analysis                 │     │
│  └───────────────────────────────────┘     │
│                                             │
└─────────────────────────────────────────────┘
```

### AWR Configuration

```yaml
# /etc/rusty-db/rusty-db.conf
[awr]
enabled = true
snapshot_interval_minutes = 60
retention_days = 30
baseline_retention_days = 90

# What to capture
capture_sql_stats = true
capture_wait_events = true
capture_io_stats = true
capture_buffer_pool_stats = true
capture_ash_aggregation = true
```

### Creating AWR Snapshots

```bash
# Create manual snapshot
curl -X POST http://localhost:8080/api/v1/awr/snapshots \
  -H "Authorization: Bearer $TOKEN"

# List snapshots
curl http://localhost:8080/api/v1/awr/snapshots \
  -H "Authorization: Bearer $TOKEN"

# Get snapshot details
curl http://localhost:8080/api/v1/awr/snapshots/12345 \
  -H "Authorization: Bearer $TOKEN"
```

### Generating AWR Reports

#### HTML Report

```bash
# Generate AWR report between two snapshots
curl -X POST http://localhost:8080/api/v1/awr/report \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "start_snapshot_id": 100,
    "end_snapshot_id": 110,
    "format": "html"
  }' > awr_report.html
```

#### Text Report

```bash
# Generate text AWR report
curl -X POST http://localhost:8080/api/v1/awr/report \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "start_snapshot_id": 100,
    "end_snapshot_id": 110,
    "format": "text"
  }' > awr_report.txt
```

#### JSON Report

```bash
# Generate JSON AWR report for programmatic analysis
curl -X POST http://localhost:8080/api/v1/awr/report \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "start_snapshot_id": 100,
    "end_snapshot_id": 110,
    "format": "json"
  }' > awr_report.json
```

### AWR Report Contents

An AWR report includes:

1. **Report Summary**
   - Snapshot information
   - Time range
   - Database version
   - Instance information

2. **Load Profile**
   - Queries per second
   - Transactions per second
   - Logical reads per second
   - Physical reads per second
   - DB time distribution

3. **Top 10 SQL by Elapsed Time**
   - SQL ID
   - Executions
   - Elapsed time
   - CPU time
   - I/O wait time
   - SQL text

4. **Top 10 SQL by CPU Time**
5. **Top 10 SQL by Buffer Gets**
6. **Top 10 SQL by Physical Reads**

7. **Wait Event Statistics**
   - Wait class breakdown
   - Top wait events
   - Average wait time
   - Wait event histogram

8. **I/O Statistics**
   - Reads/writes by file type
   - Average read/write latency
   - I/O throughput

9. **Buffer Pool Statistics**
   - Hit ratio
   - Gets and misses
   - Evictions

10. **Transaction Statistics**
    - Commits
    - Rollbacks
    - Deadlocks

### AWR Baselines

Create baselines for normal operation periods:

```bash
# Create baseline
curl -X POST http://localhost:8080/api/v1/awr/baselines \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "normal_business_hours",
    "start_snapshot_id": 100,
    "end_snapshot_id": 200,
    "description": "Typical Monday morning workload"
  }'

# Compare current performance to baseline
curl -X POST http://localhost:8080/api/v1/awr/compare \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "baseline_name": "normal_business_hours",
    "current_start_snapshot_id": 300,
    "current_end_snapshot_id": 310
  }'
```

### Trend Analysis

```bash
# Get performance trends over time
curl http://localhost:8080/api/v1/awr/trends?metric=queries_per_second&days=7 \
  -H "Authorization: Bearer $TOKEN"
```

**Response**:

```json
{
  "success": true,
  "data": {
    "metric": "queries_per_second",
    "time_range": {
      "start": "2025-12-20T00:00:00Z",
      "end": "2025-12-27T00:00:00Z"
    },
    "data_points": [
      {
        "timestamp": "2025-12-20T10:00:00Z",
        "value": 5234.56,
        "snapshot_id": 100
      },
      {
        "timestamp": "2025-12-20T11:00:00Z",
        "value": 5678.90,
        "snapshot_id": 101
      }
    ],
    "statistics": {
      "min": 3456.78,
      "max": 7890.12,
      "avg": 5678.90,
      "stddev": 456.78
    }
  }
}
```

---

## Best Practices

### Monitoring Strategy

1. **Implement Multi-Layer Monitoring**
   - Application-level: Query performance, transaction metrics
   - Database-level: Buffer pool, locks, I/O
   - System-level: CPU, memory, disk, network
   - Business-level: SLA compliance, user experience

2. **Set Appropriate Alert Thresholds**
   - Start conservative, tune based on baselines
   - Use percentiles (p95, p99) not just averages
   - Implement escalation for critical alerts
   - Avoid alert fatigue

3. **Establish Baselines**
   - Create AWR baselines for normal operation
   - Document expected performance characteristics
   - Monitor trends, not just absolute values
   - Review and update baselines quarterly

4. **Regular Review Cadence**
   - Daily: Active alerts, slow queries
   - Weekly: Performance trends, capacity
   - Monthly: AWR reports, baseline comparison
   - Quarterly: Monitoring strategy review

### Performance Monitoring

1. **Focus on Key Metrics**
   - Query latency (p95, p99)
   - Transactions per second
   - Cache hit ratio (> 95%)
   - Replication lag (< 10s)
   - Connection pool utilization (< 80%)

2. **Use ASH for Troubleshooting**
   - Capture detailed session activity
   - Analyze wait events
   - Identify resource bottlenecks
   - Track problem queries

3. **Optimize Slow Queries**
   - Set slow query threshold (1000ms)
   - Log execution plans
   - Review slow query log daily
   - Create indexes based on analysis

### Capacity Planning

1. **Monitor Growth Trends**
   - Database size growth
   - Connection count trends
   - Query volume trends
   - Resource utilization trends

2. **Proactive Scaling**
   - Alert at 75% capacity
   - Plan expansion at 80%
   - Execute before 85%

3. **Regular Capacity Reviews**
   - Monthly capacity metrics review
   - Quarterly forecast update
   - Annual capacity planning

### Security Monitoring

1. **Monitor Security Events**
   - Failed login attempts
   - Privilege escalations
   - Suspicious query patterns
   - Audit log violations

2. **Set Security Alerts**
   - > 10 failed logins per minute
   - Unauthorized privilege grants
   - After-hours administrative activity
   - Data export anomalies

### High Availability Monitoring

1. **Replication Monitoring**
   - Monitor lag continuously
   - Alert on replica failure
   - Track replication throughput
   - Verify failover readiness

2. **Cluster Health**
   - Monitor all nodes
   - Track consensus state
   - Monitor inter-node latency
   - Verify automatic failover

---

## Troubleshooting

### Common Monitoring Issues

#### Metrics Not Appearing in Prometheus

**Symptoms**: Prometheus doesn't show RustyDB metrics

**Diagnosis**:

```bash
# Check if metrics endpoint is accessible
curl http://localhost:9187/metrics

# Check if Prometheus can reach target
curl http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job=="rustydb")'

# Check Prometheus logs
journalctl -u prometheus -n 100
```

**Solutions**:

1. Verify exporter is running:
   ```bash
   systemctl status rustydb-exporter
   ```

2. Check firewall:
   ```bash
   sudo ufw allow 9187/tcp
   ```

3. Verify Prometheus configuration:
   ```yaml
   - job_name: 'rustydb'
     static_configs:
       - targets: ['localhost:9187']
   ```

#### High Alert Volume

**Symptoms**: Too many alerts, alert fatigue

**Solutions**:

1. Adjust thresholds:
   ```bash
   # Update alert rule
   curl -X PUT http://localhost:8080/api/v1/alerts/rules/threshold/high_cpu \
     -H "Authorization: Bearer $TOKEN" \
     -d '{"threshold": 85.0, "duration_seconds": 600}'
   ```

2. Implement alert suppression:
   ```bash
   # Suppress during maintenance
   curl -X POST http://localhost:8080/api/v1/alerts/suppress \
     -H "Authorization: Bearer $TOKEN" \
     -d '{"pattern": ".*", "duration_minutes": 60}'
   ```

3. Use anomaly detection for noisy metrics

#### ASH Samples Not Retained

**Symptoms**: ASH data missing for time range

**Diagnosis**:

```bash
# Check ASH configuration
curl http://localhost:8080/api/v1/ash/config \
  -H "Authorization: Bearer $TOKEN"

# Check sample count
curl http://localhost:8080/api/v1/ash/stats \
  -H "Authorization: Bearer $TOKEN"
```

**Solutions**:

1. Increase sample retention:
   ```yaml
   [ash]
   max_samples = 172800  # 48 hours at 1Hz
   ```

2. Reduce sample interval if needed:
   ```yaml
   [ash]
   sample_interval_seconds = 5  # Less frequent sampling
   ```

#### Slow Dashboard Performance

**Symptoms**: Dashboard queries slow or timeout

**Solutions**:

1. Use shorter time ranges
2. Increase dashboard aggregation interval
3. Use pre-aggregated metrics
4. Implement dashboard caching

### Diagnostic Queries

```bash
# Check monitoring system health
curl http://localhost:8080/api/v1/monitoring/health \
  -H "Authorization: Bearer $TOKEN"

# Get monitoring statistics
curl http://localhost:8080/api/v1/monitoring/stats \
  -H "Authorization: Bearer $TOKEN"

# Check alert rule performance
curl http://localhost:8080/api/v1/alerts/rules/stats \
  -H "Authorization: Bearer $TOKEN"
```

---

## Summary

RustyDB provides enterprise-grade monitoring capabilities comparable to Oracle Database, including:

- **Prometheus-compatible metrics** for integration with modern monitoring stacks
- **Active Session History (ASH)** for detailed performance analysis
- **Automatic Workload Repository (AWR)** for historical reporting and trend analysis
- **Comprehensive alerting** with threshold and anomaly detection
- **Real-time dashboards** via REST and WebSocket APIs
- **Diagnostic capabilities** for incident management and troubleshooting

### Key Takeaways

1. **Implement comprehensive monitoring** across all layers (application, database, system)
2. **Use ASH and AWR** for performance analysis and historical trending
3. **Set appropriate alerts** to detect issues before they impact users
4. **Review metrics regularly** to identify trends and capacity needs
5. **Integrate with existing tools** (Prometheus, Grafana, PagerDuty, Slack)
6. **Establish baselines** for normal operation and compare regularly
7. **Monitor proactively** to prevent issues rather than react to them

### Additional Resources

- [RustyDB Operations Guide](/docs/OPERATIONS_GUIDE.md)
- [RustyDB API Reference](/docs/API_REFERENCE.md)
- [RustyDB Performance Tuning Guide](/docs/PERFORMANCE_GUIDE.md)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)

---

**Document Version**: 1.0
**Last Updated**: 2025-12-27
**Maintained By**: RustyDB Engineering Team
**Contact**: support@rustydb.com
