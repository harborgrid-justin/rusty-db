# RustyDB Operations Guide

**Document Version**: 1.0
**Last Updated**: 2025-12-11
**Classification**: Internal
**Maintained By**: Operations Team

---

## Executive Summary

This guide provides comprehensive procedures for day-to-day operations, monitoring, maintenance, and troubleshooting of RustyDB in production environments. It is designed for database administrators, DevOps engineers, and operations teams.

---

## Table of Contents

1. [Daily Operations](#daily-operations)
2. [Health Monitoring](#health-monitoring)
3. [Performance Tuning](#performance-tuning)
4. [Backup and Recovery](#backup-and-recovery)
5. [High Availability Operations](#high-availability-operations)
6. [Disaster Recovery](#disaster-recovery)
7. [Capacity Planning](#capacity-planning)
8. [Maintenance Windows](#maintenance-windows)
9. [Log Management](#log-management)
10. [Troubleshooting](#troubleshooting)
11. [Security Operations](#security-operations)
12. [Monitoring and Alerting](#monitoring-and-alerting)

---

## Daily Operations

### Morning Health Check

Perform these checks at the start of each business day:

```bash
# 1. Check database server status
systemctl status rusty-db

# 2. Verify all instances are running
rusty-db-cli --command "SELECT instance_name, status FROM v$instance;"

# 3. Check tablespace usage
rusty-db-cli --command "SELECT tablespace_name, used_percent FROM v$tablespace_usage WHERE used_percent > 80;"

# 4. Review overnight backup status
rusty-db-cli --command "SELECT backup_type, status, start_time, end_time FROM v$backup_history WHERE start_time > NOW() - INTERVAL '24 hours';"

# 5. Check for failed jobs
rusty-db-cli --command "SELECT job_name, status, error_message FROM v$scheduled_jobs WHERE status = 'FAILED';"

# 6. Review alert log for errors
tail -100 /var/log/rusty-db/alert.log | grep -i "error\|warn"

# 7. Check replication lag (if applicable)
rusty-db-cli --command "SELECT standby_name, lag_seconds FROM v$replication_lag WHERE lag_seconds > 30;"

# 8. Verify connection pool health
rusty-db-cli --command "SELECT pool_name, active_connections, idle_connections, wait_queue_depth FROM v$connection_pools;"
```

### Daily Metrics Review

Monitor these key metrics daily:

1. **Database Performance**
   - Query response time (p50, p95, p99)
   - Transactions per second (TPS)
   - Cache hit ratio (should be > 95%)
   - Lock wait time

2. **System Resources**
   - CPU utilization (alert if > 80%)
   - Memory usage (alert if > 85%)
   - Disk I/O (IOPS and throughput)
   - Network bandwidth

3. **Storage Metrics**
   - Data file growth rate
   - Log file growth rate
   - Archive log generation rate
   - Tablespace utilization

4. **Availability Metrics**
   - Uptime percentage
   - Connection success rate
   - Replication lag
   - Cluster node health

### Routine Tasks

#### Hourly
- Monitor active sessions and long-running queries
- Check connection pool utilization
- Review real-time performance metrics

#### Daily
- Review backup completion status
- Check alert logs for errors or warnings
- Monitor disk space usage
- Review security audit logs
- Check replication status

#### Weekly
- Analyze slow query logs
- Review and archive old logs
- Check for available software updates
- Review capacity trends
- Test disaster recovery procedures (sample test)

#### Monthly
- Full disaster recovery drill
- Review and update runbooks
- Performance baseline review
- Security audit review
- Capacity planning review

---

## Health Monitoring

### System Health Commands

```bash
# Overall database health
rusty-db-cli --command "SELECT * FROM v$database_health;"

# Component health status
rusty-db-cli --command "SELECT component, status, last_check_time FROM v$component_health;"

# Resource utilization
rusty-db-cli --command "SELECT * FROM v$resource_usage;"

# Active sessions
rusty-db-cli --command "SELECT session_id, username, status, query, duration_seconds FROM v$active_sessions ORDER BY duration_seconds DESC LIMIT 20;"

# Lock contention
rusty-db-cli --command "SELECT blocker_session, blocked_session, lock_type, wait_time_seconds FROM v$lock_waits ORDER BY wait_time_seconds DESC;"

# Buffer pool statistics
rusty-db-cli --command "SELECT * FROM v$buffer_pool_stats;"
```

### Health Monitoring Thresholds

| Metric | Warning Threshold | Critical Threshold | Action |
|--------|------------------|-------------------|---------|
| CPU Usage | 75% | 90% | Scale up or optimize queries |
| Memory Usage | 80% | 90% | Add memory or tune buffer pool |
| Disk Space (Data) | 75% | 85% | Add storage or archive data |
| Disk Space (Logs) | 80% | 90% | Increase log rotation frequency |
| Cache Hit Ratio | < 90% | < 80% | Increase buffer pool size |
| Replication Lag | > 10s | > 30s | Check network or increase bandwidth |
| Connection Pool Usage | 80% | 95% | Increase pool size |
| Failed Login Attempts | > 10/min | > 50/min | Possible attack, enable IP blocking |

### Automated Health Checks

Configure automated health monitoring:

```bash
# Create health check script
cat > /usr/local/bin/rusty-db-health-check.sh << 'EOF'
#!/bin/bash

# Configuration
ALERT_EMAIL="dba-team@company.com"
METRICS_DIR="/var/lib/rusty-db/metrics"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Check CPU usage
CPU_USAGE=$(top -bn1 | grep "Cpu(s)" | sed "s/.*, *\([0-9.]*\)%* id.*/\1/" | awk '{print 100 - $1}')
if (( $(echo "$CPU_USAGE > 80" | bc -l) )); then
    echo "WARNING: High CPU usage: ${CPU_USAGE}%" | mail -s "RustyDB Alert: High CPU" $ALERT_EMAIL
fi

# Check memory usage
MEM_USAGE=$(free | grep Mem | awk '{print ($3/$2) * 100.0}')
if (( $(echo "$MEM_USAGE > 85" | bc -l) )); then
    echo "WARNING: High memory usage: ${MEM_USAGE}%" | mail -s "RustyDB Alert: High Memory" $ALERT_EMAIL
fi

# Check disk space
DISK_USAGE=$(df -h /var/lib/rusty-db | tail -1 | awk '{print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -gt 80 ]; then
    echo "WARNING: High disk usage: ${DISK_USAGE}%" | mail -s "RustyDB Alert: Low Disk Space" $ALERT_EMAIL
fi

# Check database connectivity
if ! rusty-db-cli --command "SELECT 1;" > /dev/null 2>&1; then
    echo "CRITICAL: Database connectivity failed!" | mail -s "RustyDB Alert: DB Down" $ALERT_EMAIL
fi

# Save metrics
echo "${TIMESTAMP},${CPU_USAGE},${MEM_USAGE},${DISK_USAGE}" >> ${METRICS_DIR}/health-metrics.csv
EOF

chmod +x /usr/local/bin/rusty-db-health-check.sh

# Schedule health checks every 5 minutes
echo "*/5 * * * * /usr/local/bin/rusty-db-health-check.sh" | crontab -
```

---

## Performance Tuning

### Query Performance Analysis

```bash
# Find slow queries
rusty-db-cli --command "
SELECT
    query_id,
    query_text,
    execution_count,
    avg_execution_time_ms,
    max_execution_time_ms,
    total_cpu_time_ms
FROM v$slow_queries
WHERE avg_execution_time_ms > 1000
ORDER BY avg_execution_time_ms DESC
LIMIT 20;
"

# Analyze query execution plan
rusty-db-cli --command "EXPLAIN ANALYZE <your_query_here>;"

# Check index usage
rusty-db-cli --command "
SELECT
    table_name,
    index_name,
    scans,
    rows_read,
    rows_fetched,
    last_used
FROM v$index_usage
WHERE scans = 0 AND created_at < NOW() - INTERVAL '30 days';
"

# Find missing indexes
rusty-db-cli --command "
SELECT
    table_name,
    column_names,
    estimated_improvement,
    query_count
FROM v$missing_indexes
ORDER BY estimated_improvement DESC
LIMIT 10;
"
```

### Buffer Pool Tuning

```bash
# Check buffer pool hit ratio
rusty-db-cli --command "
SELECT
    pool_name,
    buffer_hit_ratio,
    page_reads,
    page_writes,
    evictions
FROM v$buffer_pool_stats;
"

# Recommended buffer pool size based on working set
rusty-db-cli --command "SELECT recommended_buffer_pool_size_mb FROM v$tuning_advisor WHERE advisor = 'buffer_pool';"

# Update buffer pool size (requires restart)
# Edit /etc/rusty-db/rusty-db.conf:
# buffer_pool_size_mb = 8192
```

### Connection Pool Optimization

```bash
# Analyze connection pool usage
rusty-db-cli --command "
SELECT
    pool_name,
    min_connections,
    max_connections,
    active_connections,
    idle_connections,
    wait_queue_depth,
    avg_wait_time_ms,
    connection_timeouts
FROM v$connection_pool_stats;
"

# Tune connection pool settings
# Edit /etc/rusty-db/rusty-db.conf:
# max_connections = 500
# connection_pool_size = 100
# connection_timeout_seconds = 30
# idle_connection_timeout_seconds = 300
```

### I/O Performance Tuning

```bash
# Check I/O statistics
rusty-db-cli --command "
SELECT
    file_type,
    file_name,
    reads_per_sec,
    writes_per_sec,
    avg_read_latency_ms,
    avg_write_latency_ms
FROM v$file_io_stats
ORDER BY (reads_per_sec + writes_per_sec) DESC
LIMIT 20;
"

# Enable direct I/O (Linux)
# Edit /etc/rusty-db/rusty-db.conf:
# direct_io = true
# io_uring_enabled = true  # For Linux kernel 5.1+

# Use io_uring for async I/O on Linux
# Ensure kernel version >= 5.1
uname -r

# Enable prefetching for sequential scans
rusty-db-cli --command "ALTER SYSTEM SET prefetch_size = 128;"  # pages
```

### Memory Tuning

```bash
# Check memory usage breakdown
rusty-db-cli --command "
SELECT
    component,
    allocated_mb,
    used_mb,
    free_mb,
    usage_percent
FROM v$memory_usage
ORDER BY allocated_mb DESC;
"

# Configure memory allocator
# Edit /etc/rusty-db/rusty-db.conf:
# slab_allocator_enabled = true
# arena_allocator_size_mb = 512
# large_object_threshold_kb = 256
# huge_pages_enabled = true
```

### Statistics Collection

```bash
# Manually gather table statistics
rusty-db-cli --command "ANALYZE TABLE customers;"

# Gather statistics for entire schema
rusty-db-cli --command "ANALYZE SCHEMA public;"

# Configure automatic statistics gathering
rusty-db-cli --command "
ALTER SYSTEM SET auto_statistics_gather = true;
ALTER SYSTEM SET statistics_gather_interval = '1 day';
"

# View statistics staleness
rusty-db-cli --command "
SELECT
    table_name,
    last_analyzed,
    rows,
    stale_percent
FROM v$table_statistics
WHERE stale_percent > 10
ORDER BY stale_percent DESC;
"
```

---

## Backup and Recovery

### Backup Operations

#### Full Backup

```bash
# Create full database backup
rusty-db-backup --type full \
  --output /backups/rusty-db/full_$(date +%Y%m%d_%H%M%S).backup \
  --compress \
  --encrypt \
  --verify

# Full backup with parallel threads
rusty-db-backup --type full \
  --output /backups/rusty-db/full_$(date +%Y%m%d_%H%M%S).backup \
  --threads 4 \
  --compress \
  --encrypt \
  --verify

# Backup specific database
rusty-db-backup --type full \
  --database production_db \
  --output /backups/rusty-db/prod_full_$(date +%Y%m%d_%H%M%S).backup
```

#### Incremental Backup

```bash
# Create incremental backup (changes since last full or incremental)
rusty-db-backup --type incremental \
  --output /backups/rusty-db/incr_$(date +%Y%m%d_%H%M%S).backup \
  --compress \
  --encrypt

# Incremental backup with base backup reference
rusty-db-backup --type incremental \
  --base-backup /backups/rusty-db/full_20251211_000000.backup \
  --output /backups/rusty-db/incr_$(date +%Y%m%d_%H%M%S).backup
```

#### Point-in-Time Backup (PITR)

```bash
# Enable continuous archiving for PITR
rusty-db-cli --command "ALTER SYSTEM SET archive_mode = on;"
rusty-db-cli --command "ALTER SYSTEM SET archive_dest = '/var/lib/rusty-db/archive';"

# Create base backup for PITR
rusty-db-backup --type pitr-base \
  --output /backups/rusty-db/pitr_base_$(date +%Y%m%d_%H%M%S).backup

# Archive logs are automatically copied to archive_dest
```

### Backup Verification

```bash
# Verify backup integrity
rusty-db-backup --verify /backups/rusty-db/full_20251211_000000.backup

# List backup contents
rusty-db-backup --list /backups/rusty-db/full_20251211_000000.backup

# Check backup metadata
rusty-db-backup --info /backups/rusty-db/full_20251211_000000.backup
```

### Backup Schedule

Recommended backup schedule:

```bash
# Create backup script
cat > /usr/local/bin/rusty-db-backup.sh << 'EOF'
#!/bin/bash

BACKUP_DIR="/backups/rusty-db"
RETENTION_DAYS=30
DATE=$(date +%Y%m%d_%H%M%S)
DAY_OF_WEEK=$(date +%u)  # 1-7 (Monday-Sunday)

# Create backup directory if it doesn't exist
mkdir -p $BACKUP_DIR

# Full backup on Sunday (day 7), incremental on other days
if [ "$DAY_OF_WEEK" -eq 7 ]; then
    # Full backup
    rusty-db-backup --type full \
      --output $BACKUP_DIR/full_${DATE}.backup \
      --compress \
      --encrypt \
      --verify \
      --threads 4
else
    # Incremental backup
    rusty-db-backup --type incremental \
      --output $BACKUP_DIR/incr_${DATE}.backup \
      --compress \
      --encrypt \
      --verify
fi

# Clean up old backups
find $BACKUP_DIR -name "*.backup" -mtime +$RETENTION_DAYS -delete

# Upload to S3 (optional)
# aws s3 sync $BACKUP_DIR s3://company-rustydb-backups/ --storage-class GLACIER
EOF

chmod +x /usr/local/bin/rusty-db-backup.sh

# Schedule daily backups at 2 AM
echo "0 2 * * * /usr/local/bin/rusty-db-backup.sh" | crontab -
```

### Restore Operations

#### Full Restore

```bash
# Stop the database (if running)
systemctl stop rusty-db

# Restore from full backup
rusty-db-restore --input /backups/rusty-db/full_20251211_000000.backup \
  --data-dir /var/lib/rusty-db \
  --threads 4

# Start the database
systemctl start rusty-db

# Verify database integrity
rusty-db-cli --command "SELECT * FROM v$database_health;"
```

#### Point-in-Time Recovery

```bash
# Restore to specific timestamp
rusty-db-restore --input /backups/rusty-db/pitr_base_20251211_000000.backup \
  --archive-dir /var/lib/rusty-db/archive \
  --recovery-target-time "2025-12-11 14:30:00" \
  --data-dir /var/lib/rusty-db

# Restore to specific transaction ID
rusty-db-restore --input /backups/rusty-db/pitr_base_20251211_000000.backup \
  --archive-dir /var/lib/rusty-db/archive \
  --recovery-target-txid 1234567 \
  --data-dir /var/lib/rusty-db
```

#### Table-Level Restore

```bash
# Restore specific table from backup
rusty-db-restore --input /backups/rusty-db/full_20251211_000000.backup \
  --table customers \
  --data-dir /var/lib/rusty-db/temp \
  --no-start

# Export restored table
rusty-db-export --table customers \
  --input-dir /var/lib/rusty-db/temp \
  --output customers_restored.sql

# Import into production database
rusty-db-cli < customers_restored.sql
```

---

## High Availability Operations

### Cluster Status

```bash
# Check cluster health
rusty-db-cli --command "SELECT * FROM v$cluster_status;"

# View cluster nodes
rusty-db-cli --command "
SELECT
    node_id,
    node_name,
    role,
    status,
    last_heartbeat,
    replication_lag_seconds
FROM v$cluster_nodes
ORDER BY node_id;
"

# Check Raft consensus status
rusty-db-cli --command "SELECT * FROM v$raft_status;"
```

### Failover Operations

#### Manual Failover

```bash
# Initiate manual failover to standby
rusty-db-cluster --failover --target-node standby-1

# Promote standby to primary
rusty-db-cli --command "ALTER SYSTEM PROMOTE TO PRIMARY;"

# Verify new primary
rusty-db-cli --command "SELECT role, is_primary FROM v$instance;"
```

#### Switchover (Planned)

```bash
# Perform planned switchover
rusty-db-cluster --switchover --from primary-1 --to standby-1

# Steps performed automatically:
# 1. Sync standby with primary
# 2. Stop writes on primary
# 3. Wait for replication to catch up
# 4. Promote standby to primary
# 5. Reconfigure old primary as standby
```

### Replication Management

```bash
# Check replication status
rusty-db-cli --command "
SELECT
    standby_name,
    status,
    lag_seconds,
    lag_bytes,
    last_receive_time,
    last_replay_time
FROM v$replication_status;
"

# Pause replication
rusty-db-cli --command "ALTER SYSTEM PAUSE REPLICATION TO 'standby-1';"

# Resume replication
rusty-db-cli --command "ALTER SYSTEM RESUME REPLICATION TO 'standby-1';"

# Re-sync standby from primary
rusty-db-cluster --resync --standby standby-1 --from primary-1

# Add new standby
rusty-db-cluster --add-standby --name standby-2 --host 192.168.1.102
```

### Split-Brain Detection

```bash
# Check for split-brain condition
rusty-db-cli --command "SELECT * FROM v$split_brain_detection;"

# Resolve split-brain (carefully!)
# 1. Identify the authoritative primary
# 2. Demote the other node
rusty-db-cli --host 192.168.1.102 --command "ALTER SYSTEM DEMOTE TO STANDBY;"

# 3. Resync the demoted node
rusty-db-cluster --resync --standby 192.168.1.102 --from 192.168.1.101
```

---

## Disaster Recovery

### DR Readiness Check

```bash
# Verify DR site status
rusty-db-cli --host dr-site.company.com --command "SELECT * FROM v$database_health;"

# Check DR replication lag
rusty-db-cli --command "
SELECT
    standby_name,
    lag_seconds,
    estimated_data_loss_mb
FROM v$replication_status
WHERE standby_name LIKE 'dr-%';
"

# Test DR connectivity
ping -c 5 dr-site.company.com
nc -zv dr-site.company.com 5432
```

### DR Failover Procedure

```bash
# 1. Verify primary site is down
ping -c 10 primary-site.company.com

# 2. Check DR site health
rusty-db-cli --host dr-site.company.com --command "SELECT * FROM v$database_health;"

# 3. Promote DR site to primary
rusty-db-cli --host dr-site.company.com --command "ALTER SYSTEM PROMOTE TO PRIMARY;"

# 4. Update DNS/load balancer to point to DR site
# (Application-specific)

# 5. Verify application connectivity
rusty-db-cli --host dr-site.company.com --command "SELECT 1;"

# 6. Monitor for issues
rusty-db-cli --host dr-site.company.com --command "SELECT * FROM v$active_sessions;"
```

### DR Testing

```bash
# Schedule quarterly DR drill
# Create DR test plan:

# 1. Notify stakeholders
# 2. Take snapshot of DR database
# 3. Promote DR to primary (isolated network)
# 4. Run validation tests
# 5. Document results
# 6. Revert DR to standby mode
# 7. Resync DR with production

# DR test script
cat > /usr/local/bin/rusty-db-dr-test.sh << 'EOF'
#!/bin/bash

DR_HOST="dr-site.company.com"
LOG_FILE="/var/log/rusty-db/dr-test-$(date +%Y%m%d).log"

echo "Starting DR test at $(date)" >> $LOG_FILE

# 1. Create snapshot
echo "Creating DR snapshot..." >> $LOG_FILE
rusty-db-cli --host $DR_HOST --command "CREATE SNAPSHOT dr_test_snapshot;" >> $LOG_FILE 2>&1

# 2. Promote to primary (test mode)
echo "Promoting DR to primary (test mode)..." >> $LOG_FILE
rusty-db-cli --host $DR_HOST --command "ALTER SYSTEM PROMOTE TO PRIMARY TEST MODE;" >> $LOG_FILE 2>&1

# 3. Run validation queries
echo "Running validation tests..." >> $LOG_FILE
rusty-db-cli --host $DR_HOST --command "SELECT COUNT(*) FROM customers;" >> $LOG_FILE 2>&1
rusty-db-cli --host $DR_HOST --command "SELECT * FROM v$database_health;" >> $LOG_FILE 2>&1

# 4. Revert to standby
echo "Reverting to standby mode..." >> $LOG_FILE
rusty-db-cli --host $DR_HOST --command "ALTER SYSTEM DEMOTE TO STANDBY;" >> $LOG_FILE 2>&1

# 5. Restore snapshot
echo "Restoring snapshot..." >> $LOG_FILE
rusty-db-cli --host $DR_HOST --command "RESTORE SNAPSHOT dr_test_snapshot;" >> $LOG_FILE 2>&1

echo "DR test completed at $(date)" >> $LOG_FILE
EOF

chmod +x /usr/local/bin/rusty-db-dr-test.sh

# Schedule quarterly DR tests
# 0 2 1 */3 * /usr/local/bin/rusty-db-dr-test.sh
```

---

## Capacity Planning

### Capacity Metrics Collection

```bash
# Collect growth metrics
rusty-db-cli --command "
SELECT
    metric_date,
    total_data_size_gb,
    total_index_size_gb,
    total_log_size_gb,
    total_connections,
    avg_transactions_per_second
FROM v$capacity_metrics
WHERE metric_date >= NOW() - INTERVAL '90 days'
ORDER BY metric_date;
" > capacity-metrics-$(date +%Y%m%d).csv

# Analyze growth trends
rusty-db-cli --command "
SELECT
    'Data Growth' as metric,
    ROUND(AVG(daily_growth_gb), 2) as avg_daily_growth_gb,
    ROUND(AVG(daily_growth_gb) * 30, 2) as projected_monthly_gb,
    ROUND(AVG(daily_growth_gb) * 365, 2) as projected_yearly_gb
FROM (
    SELECT
        metric_date,
        total_data_size_gb - LAG(total_data_size_gb) OVER (ORDER BY metric_date) as daily_growth_gb
    FROM v$capacity_metrics
    WHERE metric_date >= NOW() - INTERVAL '90 days'
) growth_data;
"
```

### Capacity Forecasting

```bash
# Get capacity recommendations
rusty-db-cli --command "
SELECT
    resource_type,
    current_value,
    recommended_value,
    urgency,
    reason
FROM v$capacity_recommendations
ORDER BY urgency DESC;
"

# Estimate time to capacity limit
rusty-db-cli --command "
SELECT
    resource_type,
    current_usage_percent,
    days_until_80_percent,
    days_until_90_percent,
    days_until_full
FROM v$capacity_forecast;
"
```

### Capacity Expansion

```bash
# Add storage volume
# 1. Create new volume (cloud/SAN specific)
# 2. Attach to database server
# 3. Create filesystem
sudo mkfs.ext4 /dev/sdb
sudo mkdir -p /var/lib/rusty-db/data2
sudo mount /dev/sdb /var/lib/rusty-db/data2

# 4. Add tablespace on new volume
rusty-db-cli --command "
CREATE TABLESPACE data2
LOCATION '/var/lib/rusty-db/data2';
"

# 5. Move large tables to new tablespace
rusty-db-cli --command "ALTER TABLE large_table SET TABLESPACE data2;"

# Scale up compute resources (vertical scaling)
# 1. Schedule maintenance window
# 2. Stop database
systemctl stop rusty-db

# 3. Increase VM resources (cloud-specific)
# Example AWS:
# aws ec2 modify-instance-attribute --instance-id i-1234567890abcdef0 --instance-type m5.4xlarge

# 4. Update database configuration
# Edit /etc/rusty-db/rusty-db.conf
# max_connections = 1000
# buffer_pool_size_mb = 16384

# 5. Start database
systemctl start rusty-db

# 6. Verify new resources
rusty-db-cli --command "SELECT * FROM v$resource_usage;"
```

---

## Maintenance Windows

### Pre-Maintenance Checklist

```bash
# 1. Verify backup is recent
rusty-db-cli --command "
SELECT backup_type, status, end_time
FROM v$backup_history
ORDER BY end_time DESC
LIMIT 1;
"

# 2. Create maintenance snapshot
rusty-db-cli --command "CREATE SNAPSHOT pre_maintenance_$(date +%Y%m%d_%H%M%S);"

# 3. Notify users
# Send notification to application teams

# 4. Enable maintenance mode
rusty-db-cli --command "ALTER SYSTEM SET maintenance_mode = 'read_only';"

# 5. Flush all pending transactions
rusty-db-cli --command "CHECKPOINT;"

# 6. Document current configuration
rusty-db-cli --command "SELECT * FROM v$configuration;" > config-backup-$(date +%Y%m%d).txt
```

### Common Maintenance Tasks

#### Schema Changes

```bash
# Add index (online)
rusty-db-cli --command "CREATE INDEX CONCURRENTLY idx_customers_email ON customers(email);"

# Add column with default (use CONCURRENTLY for large tables)
rusty-db-cli --command "ALTER TABLE customers ADD COLUMN loyalty_points INT DEFAULT 0;"

# Partition large table
rusty-db-cli --command "
ALTER TABLE orders PARTITION BY RANGE (order_date) (
    PARTITION orders_2024 VALUES LESS THAN ('2025-01-01'),
    PARTITION orders_2025 VALUES LESS THAN ('2026-01-01')
);
"
```

#### Reindexing

```bash
# Rebuild fragmented indexes
rusty-db-cli --command "REINDEX TABLE customers;"

# Rebuild all indexes in schema
rusty-db-cli --command "REINDEX SCHEMA public;"

# Rebuild concurrently (no locks)
rusty-db-cli --command "REINDEX INDEX CONCURRENTLY idx_customers_email;"
```

#### Table Maintenance

```bash
# Analyze tables for statistics
rusty-db-cli --command "ANALYZE TABLE customers;"

# Vacuum to reclaim space (if using MVCC)
rusty-db-cli --command "VACUUM FULL customers;"

# Update statistics
rusty-db-cli --command "UPDATE STATISTICS customers;"
```

### Post-Maintenance Checklist

```bash
# 1. Verify database health
rusty-db-cli --command "SELECT * FROM v$database_health;"

# 2. Disable maintenance mode
rusty-db-cli --command "ALTER SYSTEM SET maintenance_mode = 'normal';"

# 3. Run smoke tests
rusty-db-cli --command "SELECT COUNT(*) FROM customers;"
rusty-db-cli --command "SELECT * FROM v$active_sessions;"

# 4. Monitor for errors
tail -f /var/log/rusty-db/alert.log

# 5. Verify application connectivity
# Run application-specific tests

# 6. Notify users of completion

# 7. Document changes
# Update runbook/wiki
```

---

## Log Management

### Log File Locations

```
/var/log/rusty-db/
├── alert.log              # Database alerts and errors
├── audit.log              # Security audit log
├── slow-query.log         # Slow query log
├── performance.log        # Performance metrics
├── replication.log        # Replication events
├── backup.log             # Backup/restore operations
└── cluster.log            # Cluster events
```

### Log Rotation Configuration

```bash
# Configure logrotate
cat > /etc/logrotate.d/rusty-db << 'EOF'
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
EOF

# Test logrotate configuration
logrotate -d /etc/logrotate.d/rusty-db

# Force rotation
logrotate -f /etc/logrotate.d/rusty-db
```

### Log Analysis

```bash
# Find errors in alert log
grep -i "error\|fatal\|panic" /var/log/rusty-db/alert.log | tail -50

# Analyze slow queries
cat /var/log/rusty-db/slow-query.log | \
  grep "Query time:" | \
  awk '{print $3}' | \
  sort -rn | \
  head -20

# Count errors by type
grep "ERROR" /var/log/rusty-db/alert.log | \
  awk -F': ' '{print $2}' | \
  sort | \
  uniq -c | \
  sort -rn

# Security audit log analysis
grep "FAILED" /var/log/rusty-db/audit.log | \
  awk '{print $5}' | \
  sort | \
  uniq -c | \
  sort -rn
```

### Centralized Logging (ELK Stack)

```bash
# Install Filebeat
curl -L -O https://artifacts.elastic.co/downloads/beats/filebeat/filebeat-8.11.0-amd64.deb
sudo dpkg -i filebeat-8.11.0-amd64.deb

# Configure Filebeat for RustyDB logs
cat > /etc/filebeat/filebeat.yml << 'EOF'
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/rusty-db/*.log
  fields:
    application: rustydb
    environment: production

output.elasticsearch:
  hosts: ["elasticsearch.company.com:9200"]
  index: "rustydb-logs-%{+yyyy.MM.dd}"

setup.kibana:
  host: "kibana.company.com:5601"
EOF

# Start Filebeat
systemctl enable filebeat
systemctl start filebeat
```

---

## Troubleshooting

### Common Issues and Solutions

#### Issue: Database Won't Start

```bash
# Check system logs
journalctl -u rusty-db -n 100

# Check alert log
tail -100 /var/log/rusty-db/alert.log

# Verify file permissions
ls -la /var/lib/rusty-db/
chown -R rustydb:rustydb /var/lib/rusty-db/

# Check disk space
df -h

# Verify configuration file
rusty-db --check-config /etc/rusty-db/rusty-db.conf

# Start in single-user mode for recovery
rusty-db --single-user --data-dir /var/lib/rusty-db
```

#### Issue: High CPU Usage

```bash
# Find expensive queries
rusty-db-cli --command "
SELECT
    session_id,
    username,
    query,
    cpu_time_ms,
    execution_time_ms
FROM v$active_sessions
ORDER BY cpu_time_ms DESC
LIMIT 10;
"

# Kill expensive query
rusty-db-cli --command "KILL QUERY session_id;"

# Check for missing indexes
rusty-db-cli --command "SELECT * FROM v$missing_indexes;"

# Update statistics
rusty-db-cli --command "ANALYZE SCHEMA public;"
```

#### Issue: High Memory Usage

```bash
# Check memory breakdown
rusty-db-cli --command "SELECT * FROM v$memory_usage;"

# Check for memory leaks
rusty-db-cli --command "SELECT * FROM v$memory_leaks;"

# Reduce buffer pool size temporarily
rusty-db-cli --command "ALTER SYSTEM SET buffer_pool_size_mb = 4096;"

# Restart database (if necessary)
systemctl restart rusty-db
```

#### Issue: Connection Refused

```bash
# Check if service is running
systemctl status rusty-db

# Check if port is listening
netstat -tlnp | grep 5432
lsof -i :5432

# Check firewall rules
sudo iptables -L -n | grep 5432
sudo ufw status | grep 5432

# Test local connectivity
telnet localhost 5432

# Check connection limits
rusty-db-cli --command "SELECT * FROM v$connection_pool_stats;"
```

#### Issue: Replication Lag

```bash
# Check replication status
rusty-db-cli --command "SELECT * FROM v$replication_status;"

# Check network connectivity
ping standby-host
traceroute standby-host

# Check standby resources
ssh standby-host "top -bn1"

# Check for long-running transactions on primary
rusty-db-cli --command "
SELECT * FROM v$active_sessions
WHERE duration_seconds > 300
ORDER BY duration_seconds DESC;
"

# Increase replication bandwidth
rusty-db-cli --command "ALTER SYSTEM SET replication_bandwidth_mbps = 1000;"
```

#### Issue: Slow Queries

```bash
# Get query execution plan
rusty-db-cli --command "EXPLAIN ANALYZE <slow_query>;"

# Check for missing indexes
rusty-db-cli --command "SELECT * FROM v$missing_indexes WHERE table_name = 'your_table';"

# Check table statistics
rusty-db-cli --command "SELECT * FROM v$table_statistics WHERE table_name = 'your_table';"

# Update statistics
rusty-db-cli --command "ANALYZE TABLE your_table;"

# Create recommended index
rusty-db-cli --command "CREATE INDEX idx_your_table_column ON your_table(column) CONCURRENTLY;"
```

### Diagnostic Data Collection

```bash
# Create diagnostic bundle
cat > /usr/local/bin/collect-diagnostics.sh << 'EOF'
#!/bin/bash

DIAG_DIR="/tmp/rustydb-diagnostics-$(date +%Y%m%d_%H%M%S)"
mkdir -p $DIAG_DIR

# System information
echo "Collecting system information..."
uname -a > $DIAG_DIR/system-info.txt
cat /etc/os-release >> $DIAG_DIR/system-info.txt
free -h > $DIAG_DIR/memory-info.txt
df -h > $DIAG_DIR/disk-info.txt
top -bn1 > $DIAG_DIR/top-snapshot.txt

# Database logs
echo "Collecting database logs..."
tail -1000 /var/log/rusty-db/alert.log > $DIAG_DIR/alert.log
tail -1000 /var/log/rusty-db/slow-query.log > $DIAG_DIR/slow-query.log

# Database statistics
echo "Collecting database statistics..."
rusty-db-cli --command "SELECT * FROM v$database_health;" > $DIAG_DIR/database-health.txt
rusty-db-cli --command "SELECT * FROM v$active_sessions;" > $DIAG_DIR/active-sessions.txt
rusty-db-cli --command "SELECT * FROM v$resource_usage;" > $DIAG_DIR/resource-usage.txt
rusty-db-cli --command "SELECT * FROM v$configuration;" > $DIAG_DIR/configuration.txt

# Network information
echo "Collecting network information..."
netstat -tlnp > $DIAG_DIR/netstat.txt
ss -s > $DIAG_DIR/socket-stats.txt

# Create tarball
echo "Creating diagnostic tarball..."
tar -czf ${DIAG_DIR}.tar.gz -C /tmp $(basename $DIAG_DIR)

echo "Diagnostic data collected: ${DIAG_DIR}.tar.gz"
EOF

chmod +x /usr/local/bin/collect-diagnostics.sh

# Run diagnostic collection
/usr/local/bin/collect-diagnostics.sh
```

---

## Security Operations

### Security Monitoring

```bash
# Check failed login attempts
rusty-db-cli --command "
SELECT
    username,
    ip_address,
    failed_attempts,
    last_failed_attempt
FROM v$failed_logins
WHERE last_failed_attempt > NOW() - INTERVAL '1 hour'
ORDER BY failed_attempts DESC;
"

# Review audit log
rusty-db-cli --command "
SELECT
    timestamp,
    username,
    action,
    object_name,
    status
FROM v$audit_log
WHERE timestamp > NOW() - INTERVAL '24 hours'
AND status = 'DENIED'
ORDER BY timestamp DESC;
"

# Check for suspicious queries
rusty-db-cli --command "
SELECT
    username,
    query,
    threat_score,
    threat_category
FROM v$suspicious_queries
WHERE threat_score > 70
ORDER BY threat_score DESC;
"
```

### Security Incident Response

```bash
# Lock compromised account
rusty-db-cli --command "ALTER USER suspicious_user ACCOUNT LOCK;"

# Terminate active sessions
rusty-db-cli --command "
SELECT terminate_session(session_id)
FROM v$active_sessions
WHERE username = 'suspicious_user';
"

# Rotate encryption keys
rusty-db-cli --command "ALTER SYSTEM ROTATE ENCRYPTION KEYS;"

# Enable enhanced auditing
rusty-db-cli --command "ALTER SYSTEM SET audit_level = 'verbose';"
```

### Access Control Management

```bash
# Create new user
rusty-db-cli --command "CREATE USER newuser WITH PASSWORD 'secure_password';"

# Grant privileges
rusty-db-cli --command "GRANT SELECT, INSERT ON customers TO newuser;"

# Create role
rusty-db-cli --command "CREATE ROLE analyst;"
rusty-db-cli --command "GRANT SELECT ON ALL TABLES IN SCHEMA public TO analyst;"

# Assign role to user
rusty-db-cli --command "GRANT ROLE analyst TO newuser;"

# Review user privileges
rusty-db-cli --command "SELECT * FROM v$user_privileges WHERE username = 'newuser';"
```

---

## Monitoring and Alerting

### Prometheus Integration

```bash
# Install Prometheus exporter
wget https://github.com/rustydb/rustydb_exporter/releases/download/v1.0.0/rustydb_exporter-linux-amd64
chmod +x rustydb_exporter-linux-amd64
sudo mv rustydb_exporter-linux-amd64 /usr/local/bin/rustydb_exporter

# Create systemd service
cat > /etc/systemd/system/rustydb-exporter.service << 'EOF'
[Unit]
Description=RustyDB Prometheus Exporter
After=network.target

[Service]
Type=simple
User=rustydb
ExecStart=/usr/local/bin/rustydb_exporter \
  --database.dsn="postgresql://monitor:password@localhost:5432/rustydb" \
  --web.listen-address=:9187
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# Start exporter
systemctl daemon-reload
systemctl enable rustydb-exporter
systemctl start rustydb-exporter

# Configure Prometheus to scrape metrics
# Add to prometheus.yml:
# scrape_configs:
#   - job_name: 'rustydb'
#     static_configs:
#       - targets: ['localhost:9187']
```

### Grafana Dashboards

Key metrics to monitor:

1. **Database Overview**
   - Uptime
   - Transactions per second
   - Active connections
   - Cache hit ratio
   - Query response time (p50, p95, p99)

2. **Resource Usage**
   - CPU usage
   - Memory usage
   - Disk I/O (IOPS, throughput)
   - Network traffic

3. **Replication**
   - Replication lag
   - Standby status
   - Replication bandwidth

4. **Security**
   - Failed login attempts
   - Privilege violations
   - Threat scores

5. **Performance**
   - Slow queries
   - Lock wait time
   - Buffer pool efficiency
   - Index usage

### Alert Rules

```yaml
# Example Prometheus alert rules
groups:
  - name: rustydb_alerts
    rules:
      - alert: HighCPUUsage
        expr: rustydb_cpu_usage > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage on {{ $labels.instance }}"
          description: "CPU usage is {{ $value }}%"

      - alert: ReplicationLag
        expr: rustydb_replication_lag_seconds > 30
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High replication lag on {{ $labels.standby }}"
          description: "Replication lag is {{ $value }} seconds"

      - alert: LowDiskSpace
        expr: rustydb_disk_free_percent < 20
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low disk space on {{ $labels.instance }}"
          description: "Only {{ $value }}% disk space remaining"

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

## Appendix

### Useful Commands Reference

```bash
# Start/Stop/Restart Database
systemctl start rusty-db
systemctl stop rusty-db
systemctl restart rusty-db
systemctl status rusty-db

# View Logs
journalctl -u rusty-db -f
tail -f /var/log/rusty-db/alert.log

# Connect to Database
rusty-db-cli
rusty-db-cli --host remote-host --port 5432 --user admin

# Backup/Restore
rusty-db-backup --type full --output /backups/full.backup
rusty-db-restore --input /backups/full.backup

# Cluster Management
rusty-db-cluster --status
rusty-db-cluster --failover --target standby-1
rusty-db-cluster --add-standby --name standby-2

# Performance
rusty-db-cli --command "EXPLAIN ANALYZE <query>;"
rusty-db-cli --command "SELECT * FROM v$slow_queries;"
```

### Configuration Parameters

Key configuration parameters in `/etc/rusty-db/rusty-db.conf`:

```ini
# Connection Settings
port = 5432
max_connections = 500
connection_timeout_seconds = 30

# Memory Settings
buffer_pool_size_mb = 8192
slab_allocator_enabled = true
huge_pages_enabled = true

# Storage Settings
data_directory = /var/lib/rusty-db
page_size = 4096
direct_io = true

# Performance Settings
prefetch_size = 128
parallel_workers = 4
jit_enabled = true

# Replication Settings
replication_mode = async
archive_mode = on
archive_dest = /var/lib/rusty-db/archive

# Security Settings
ssl_enabled = true
ssl_cert_file = /etc/rusty-db/server.crt
ssl_key_file = /etc/rusty-db/server.key
encryption_at_rest = true
audit_enabled = true

# Logging Settings
log_level = info
log_directory = /var/log/rusty-db
slow_query_threshold_ms = 1000
```

---

**Document Maintained By**: Database Operations Team
**Contact**: dba-team@company.com
**Last Review**: 2025-12-11
**Next Review**: 2026-01-11
