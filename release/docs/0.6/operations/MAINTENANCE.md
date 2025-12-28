# RustyDB v0.6.0 - Maintenance Guide

**Document Version**: 1.0
**Release**: v0.6.0
**Last Updated**: 2025-12-28
**Classification**: Enterprise Operations

---

## Table of Contents

1. [Maintenance Overview](#maintenance-overview)
2. [Routine Maintenance](#routine-maintenance)
3. [Maintenance Operations](#maintenance-operations)
4. [Performance Maintenance](#performance-maintenance)
5. [Schema Maintenance](#schema-maintenance)
6. [Maintenance Windows](#maintenance-windows)
7. [Upgrade Procedures](#upgrade-procedures)
8. [Capacity Management](#capacity-management)

---

## Maintenance Overview

### Maintenance Categories

1. **Routine Maintenance**: Regular tasks (daily/weekly/monthly)
2. **Performance Maintenance**: Optimization and tuning
3. **Schema Maintenance**: Indexes, constraints, partitions
4. **Emergency Maintenance**: Urgent fixes
5. **Planned Upgrades**: Version upgrades

### Maintenance Schedule

| Task | Frequency | Duration | Downtime |
|------|-----------|----------|----------|
| Log rotation | Daily | 1 min | No |
| Statistics update | Daily | 5-15 min | No |
| Vacuum (auto) | Daily | Varies | No |
| Index maintenance | Weekly | 30-60 min | No* |
| Full vacuum | Monthly | 1-4 hours | Partial* |
| Major upgrade | Quarterly/Annually | 2-8 hours | Yes |

*Using CONCURRENTLY option

---

## Routine Maintenance

### Daily Tasks

**1. Health Check**:
```bash
#!/bin/bash
# Daily health check script

# Check service status
if ! systemctl is-active --quiet rustydb; then
    echo "ERROR: RustyDB service is not running"
    exit 1
fi

# Check database connectivity
if ! rusty-db-cli --command "SELECT 1;" > /dev/null 2>&1; then
    echo "ERROR: Cannot connect to database"
    exit 1
fi

# Check disk space
DISK_USAGE=$(df -h /var/lib/rustydb | tail -1 | awk '{print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -gt 80 ]; then
    echo "WARNING: Disk usage is ${DISK_USAGE}%"
fi

# Check replication lag (if applicable)
rusty-db-cli --command "
SELECT standby_name, lag_seconds
FROM v$replication_lag
WHERE lag_seconds > 30;
"

echo "Daily health check completed successfully"
```

**2. Backup Verification**:
```bash
# Verify overnight backup completed
rusty-db-cli --command "
SELECT backup_type, status, end_time
FROM v$backup_history
WHERE start_time > NOW() - INTERVAL '24 hours'
ORDER BY start_time DESC
LIMIT 1;
"
```

**3. Log Review**:
```bash
# Check for errors in last 24 hours
sudo journalctl -u rustydb --since "24 hours ago" | grep -i error

# Review slow queries
tail -100 /var/lib/rustydb/instances/default/logs/slow-query.log
```

**4. Statistics Update**:
```bash
# Update table statistics
rusty-db-cli --command "ANALYZE ALL;"
```

**REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/maintenance \
  -H "Content-Type: application/json" \
  -d '{"operation":"analyze","tables":[]}'
```

### Weekly Tasks

**1. Performance Review**:
```sql
-- Top 10 slowest queries (last week)
SELECT
    query_id,
    query_text,
    avg_execution_time_ms,
    execution_count,
    total_cpu_time_ms
FROM v$slow_queries
WHERE last_execution > NOW() - INTERVAL '7 days'
ORDER BY avg_execution_time_ms DESC
LIMIT 10;

-- Unused indexes
SELECT
    table_name,
    index_name,
    scans,
    last_used
FROM v$index_usage
WHERE scans = 0
AND created_at < NOW() - INTERVAL '30 days';
```

**2. Index Maintenance**:
```bash
# Rebuild fragmented indexes
rusty-db-cli --command "REINDEX INDEX CONCURRENTLY idx_customers_email;"

# Or rebuild all indexes for a table
rusty-db-cli --command "REINDEX TABLE CONCURRENTLY customers;"
```

**3. Log Archive**:
```bash
# Archive old logs
cd /var/lib/rustydb/instances/default/logs
gzip rustydb.log.7
mv rustydb.log.*.gz /archive/logs/
```

**4. Capacity Review**:
```sql
-- Database growth
SELECT
    metric_date,
    total_data_size_gb,
    total_data_size_gb - LAG(total_data_size_gb) OVER (ORDER BY metric_date) as weekly_growth_gb
FROM v$capacity_metrics
WHERE metric_date >= NOW() - INTERVAL '30 days'
ORDER BY metric_date DESC;
```

### Monthly Tasks

**1. Full Statistics Gather**:
```bash
rusty-db-cli --command "ANALYZE SCHEMA public VERBOSE;"
```

**2. Vacuum Full** (scheduled maintenance window):
```bash
# Check tables needing vacuum
rusty-db-cli --command "
SELECT
    schemaname,
    tablename,
    n_dead_tup,
    n_live_tup,
    ROUND(100.0 * n_dead_tup / NULLIF(n_live_tup + n_dead_tup, 0), 2) as dead_ratio
FROM pg_stat_user_tables
WHERE n_dead_tup > 10000
AND n_live_tup > 0
ORDER BY dead_ratio DESC;
"

# Vacuum specific tables
rusty-db-cli --command "VACUUM FULL ANALYZE customers;"
```

**3. Security Audit**:
```sql
-- Review user permissions
SELECT
    username,
    roles,
    last_login,
    enabled
FROM v$users
WHERE last_login < NOW() - INTERVAL '90 days'
OR enabled = false;

-- Review failed login attempts
SELECT
    username,
    ip_address,
    COUNT(*) as failed_attempts,
    MAX(timestamp) as last_attempt
FROM v$failed_logins
WHERE timestamp > NOW() - INTERVAL '30 days'
GROUP BY username, ip_address
HAVING COUNT(*) > 10
ORDER BY failed_attempts DESC;
```

**4. DR Test**:
```bash
# Test DR restoration procedures
# See BACKUP_RECOVERY.md for full DR testing procedures

# Verify DR site connectivity
ping -c 5 dr-site.company.com

# Check DR replication lag
rusty-db-cli --command "
SELECT standby_name, lag_seconds
FROM v$replication_status
WHERE standby_name LIKE 'dr-%';
"
```

---

## Maintenance Operations

### VACUUM Operations

**Purpose**: Reclaim storage from dead tuples (MVCC)

**Types**:
- **VACUUM**: Marks space as reusable, doesn't return to OS
- **VACUUM FULL**: Reclaims space, returns to OS (locks table)

**Regular VACUUM** (no table lock):
```sql
VACUUM customers;
VACUUM ANALYZE customers;  -- Also updates statistics
```

**VACUUM FULL** (exclusive lock required):
```sql
VACUUM FULL customers;
```

**Auto-Vacuum Configuration** (`conf/rustydb.toml`):
```toml
[maintenance]
auto_vacuum = true
vacuum_threshold_percent = 20  # Vacuum when 20% dead tuples
```

**REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/maintenance \
  -H "Content-Type: application/json" \
  -d '{
    "operation":"vacuum",
    "tables":["customers","orders"]
  }'
```

### ANALYZE Operations

**Purpose**: Update table statistics for query optimizer

**Commands**:
```sql
-- Analyze single table
ANALYZE customers;

-- Analyze all tables in schema
ANALYZE SCHEMA public;

-- Analyze entire database
ANALYZE ALL;
```

**Auto-Analyze Configuration**:
```toml
[maintenance]
auto_statistics_gather = true
statistics_gather_interval = "1 day"
```

### REINDEX Operations

**Purpose**: Rebuild indexes to remove bloat and fix corruption

**Commands**:
```sql
-- Reindex single index (locks table)
REINDEX INDEX idx_customers_email;

-- Reindex single index (concurrent, no lock)
REINDEX INDEX CONCURRENTLY idx_customers_email;

-- Reindex entire table
REINDEX TABLE customers;

-- Reindex entire schema
REINDEX SCHEMA public;
```

**When to Reindex**:
- Index bloat > 30%
- After major data changes
- Query performance degradation
- After corruption detection

**Check Index Bloat**:
```sql
SELECT
    schemaname,
    tablename,
    indexname,
    pg_size_pretty(pg_relation_size(indexrelid)) as index_size,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY pg_relation_size(indexrelid) DESC
LIMIT 20;
```

### CHECKPOINT Operations

**Purpose**: Force write of dirty pages to disk

**Commands**:
```sql
CHECKPOINT;
```

**REST API**:
```bash
curl -X POST http://localhost:8080/api/v1/admin/maintenance \
  -H "Content-Type: application/json" \
  -d '{"operation":"checkpoint"}'
```

**Auto-Checkpoint Configuration**:
```toml
[wal]
checkpoint_interval_ms = 60000  # 1 minute
```

---

## Performance Maintenance

### Query Optimization

**1. Identify Slow Queries**:
```sql
SELECT
    query_id,
    query_text,
    calls,
    total_time,
    mean_time,
    max_time,
    stddev_time
FROM v$pg_stat_statements
WHERE mean_time > 100  -- ms
ORDER BY total_time DESC
LIMIT 20;
```

**2. Analyze Query Plan**:
```sql
EXPLAIN ANALYZE SELECT * FROM customers WHERE email = 'user@example.com';
```

**3. Create Missing Indexes**:
```sql
-- Find missing indexes
SELECT
    table_name,
    column_names,
    estimated_improvement,
    query_count
FROM v$missing_indexes
ORDER BY estimated_improvement DESC
LIMIT 10;

-- Create index
CREATE INDEX CONCURRENTLY idx_customers_email ON customers(email);
```

**4. Update Statistics**:
```sql
ANALYZE customers;
```

### Index Maintenance

**Find Unused Indexes**:
```sql
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_stat_user_indexes
WHERE idx_scan = 0
AND indexrelname NOT LIKE 'pg_toast%'
ORDER BY pg_relation_size(indexrelid) DESC;
```

**Drop Unused Indexes**:
```sql
-- Review before dropping!
DROP INDEX CONCURRENTLY idx_unused_index;
```

### Buffer Pool Tuning

**Check Buffer Pool Statistics**:
```sql
SELECT
    pool_name,
    buffer_hit_ratio,
    page_reads,
    page_writes,
    evictions
FROM v$buffer_pool_stats;
```

**Adjust Buffer Pool Size** (`conf/rustydb.toml`):
```toml
[storage]
buffer_pool_pages = 262144  # 1 GB with 4KB pages
```

**Recommended Calculation**:
```
buffer_pool_pages = (Available_Memory_MB * 0.25 * 1024) / page_size_kb

Example:
16 GB RAM
25% for buffer pool = 4 GB = 4096 MB
4096 MB * 1024 KB / 4 KB = 1,048,576 pages
```

---

## Schema Maintenance

### Adding Columns

**Online (No Downtime)**:
```sql
-- Add column with default value
ALTER TABLE customers ADD COLUMN loyalty_points INT DEFAULT 0;

-- Add nullable column
ALTER TABLE customers ADD COLUMN preferences JSONB;
```

**Best Practices**:
- Use `DEFAULT` for non-null columns
- Add nullable columns for large tables
- Use `NOT NULL` constraint later after backfilling

### Creating Indexes

**Online Index Creation**:
```sql
CREATE INDEX CONCURRENTLY idx_customers_email ON customers(email);
```

**Best Practices**:
- Always use `CONCURRENTLY` for production
- Monitor disk space (needs 2x index size)
- Create during low-traffic periods
- Check for duplicate indexes first

### Partitioning Maintenance

**Add New Partition**:
```sql
ALTER TABLE orders ADD PARTITION orders_2026
VALUES LESS THAN ('2027-01-01');
```

**Drop Old Partition**:
```sql
-- Detach first (fast)
ALTER TABLE orders DETACH PARTITION orders_2020;

-- Then drop (can be done asynchronously)
DROP TABLE orders_2020;
```

**Partition Statistics**:
```sql
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
FROM pg_tables
WHERE tablename LIKE 'orders_%'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

---

## Maintenance Windows

### Planning Maintenance Window

**Checklist**:
- [ ] Schedule during low-traffic period
- [ ] Notify stakeholders (1 week advance)
- [ ] Create pre-maintenance backup
- [ ] Document current configuration
- [ ] Prepare rollback plan
- [ ] Test changes in staging
- [ ] Prepare monitoring dashboard

### Pre-Maintenance Checklist

```bash
#!/bin/bash
# Pre-maintenance checklist script

echo "=== Pre-Maintenance Checklist ==="

# 1. Verify recent backup
echo "1. Checking latest backup..."
rusty-db-cli --command "
SELECT backup_type, status, end_time
FROM v$backup_history
WHERE end_time > NOW() - INTERVAL '24 hours'
ORDER BY end_time DESC
LIMIT 1;
"

# 2. Create maintenance snapshot
echo "2. Creating maintenance snapshot..."
rusty-db-cli --command "CREATE SNAPSHOT pre_maintenance_$(date +%Y%m%d_%H%M%S);"

# 3. Document current configuration
echo "3. Backing up configuration..."
cp /var/lib/rustydb/instances/default/conf/rustydb.toml \
   /var/lib/rustydb/instances/default/conf/rustydb.toml.$(date +%Y%m%d_%H%M%S).bak

# 4. Record current statistics
echo "4. Recording current statistics..."
rusty-db-cli --command "SELECT * FROM v$database_health;" > /tmp/pre-maintenance-health.txt
rusty-db-cli --command "SELECT * FROM v$resource_usage;" > /tmp/pre-maintenance-resources.txt

# 5. Flush pending transactions
echo "5. Flushing pending transactions..."
rusty-db-cli --command "CHECKPOINT;"

# 6. Set maintenance mode (optional)
echo "6. Enabling maintenance mode (read-only)..."
rusty-db-cli --command "ALTER SYSTEM SET maintenance_mode = 'read_only';"

echo "=== Pre-maintenance checklist completed ==="
```

### Post-Maintenance Checklist

```bash
#!/bin/bash
# Post-maintenance checklist script

echo "=== Post-Maintenance Checklist ==="

# 1. Verify database health
echo "1. Checking database health..."
rusty-db-cli --command "SELECT * FROM v$database_health;"

# 2. Disable maintenance mode
echo "2. Disabling maintenance mode..."
rusty-db-cli --command "ALTER SYSTEM SET maintenance_mode = 'normal';"

# 3. Run smoke tests
echo "3. Running smoke tests..."
rusty-db-cli --command "SELECT COUNT(*) FROM customers;"
rusty-db-cli --command "SELECT COUNT(*) FROM orders;"

# 4. Verify replication (if applicable)
echo "4. Checking replication status..."
rusty-db-cli --command "SELECT * FROM v$replication_status;"

# 5. Monitor performance
echo "5. Checking performance metrics..."
curl -s http://localhost:8080/api/v1/stats/performance | jq

# 6. Check for errors
echo "6. Checking for errors in logs..."
sudo journalctl -u rustydb --since "5 minutes ago" | grep -i error

echo "=== Post-maintenance checklist completed ==="
```

---

## Upgrade Procedures

### Minor Version Upgrade (e.g., 0.6.0 → 0.6.1)

**Steps**:

**1. Pre-Upgrade**:
```bash
# Create full backup
rusty-db-backup --type full --output /backups/pre-upgrade-0.6.1.backup --verify

# Review release notes
curl https://github.com/rustydb/rusty-db/releases/tag/v0.6.1
```

**2. Upgrade**:
```bash
# Download new version
wget https://github.com/rustydb/rusty-db/releases/download/v0.6.1/rustydb-0.6.1-linux-x86_64.tar.gz

# Extract
tar -xzf rustydb-0.6.1-linux-x86_64.tar.gz

# Stop service
sudo systemctl stop rustydb

# Backup old binaries
sudo mv /opt/rustydb/current /opt/rustydb/0.6.0

# Install new binaries
sudo cp -r rustydb-0.6.1-linux-x86_64 /opt/rustydb/0.6.1
sudo ln -sf /opt/rustydb/0.6.1 /opt/rustydb/current

# Start service
sudo systemctl start rustydb
```

**3. Verify**:
```bash
# Check version
rusty-db-cli --command "SELECT version();"

# Check health
rusty-db-cli --command "SELECT * FROM v$database_health;"

# Monitor logs
sudo journalctl -u rustydb -f
```

### Major Version Upgrade (e.g., 0.6.x → 1.0.0)

**Requires More Planning**:

**1. Test in Staging**:
- Complete upgrade in staging environment
- Run full test suite
- Measure performance impact
- Document issues and resolutions

**2. Schedule Downtime**:
- Plan for 2-8 hours downtime
- Schedule during low-traffic period
- Notify all stakeholders

**3. Upgrade Steps**:
```bash
# Full backup with verification
rusty-db-backup --type full --output /backups/pre-major-upgrade-1.0.backup --verify

# Export schema
rusty-db-export --schema-only --output schema-0.6.sql

# Stop service
sudo systemctl stop rustydb

# Run upgrade script (version-specific)
rusty-db-upgrade --from 0.6 --to 1.0 \
  --data-dir /var/lib/rustydb/instances/default/data \
  --config /var/lib/rustydb/instances/default/conf/rustydb.toml

# Start service
sudo systemctl start rustydb

# Run post-upgrade tasks
rusty-db-cli --command "ANALYZE ALL;"
rusty-db-cli --command "REINDEX SCHEMA public;"
```

**4. Rollback Plan**:
```bash
# If upgrade fails:
sudo systemctl stop rustydb

# Restore backup
rusty-db-restore --input /backups/pre-major-upgrade-1.0.backup \
  --data-dir /var/lib/rustydb/instances/default/data \
  --force

# Restore old binaries
sudo ln -sf /opt/rustydb/0.6.0 /opt/rustydb/current

# Start service
sudo systemctl start rustydb
```

### Rolling Upgrade (Zero Downtime)

For clustered deployments:

**1. Upgrade Standby Nodes**:
```bash
# On standby-1
sudo systemctl stop rustydb
# Install new version
sudo systemctl start rustydb

# Verify replication
rusty-db-cli --host primary --command "SELECT * FROM v$replication_status;"
```

**2. Failover to Upgraded Standby**:
```bash
rusty-db-cluster --switchover --from primary --to standby-1
```

**3. Upgrade Old Primary**:
```bash
# On old primary (now standby)
sudo systemctl stop rustydb
# Install new version
sudo systemctl start rustydb
```

**4. Verify All Nodes**:
```bash
rusty-db-cli --command "
SELECT node_name, version, status
FROM v$cluster_nodes;
"
```

---

## Capacity Management

### Monitoring Capacity

**Database Size**:
```sql
SELECT
    pg_database.datname as database,
    pg_size_pretty(pg_database_size(pg_database.datname)) as size
FROM pg_database
ORDER BY pg_database_size(pg_database.datname) DESC;
```

**Table Sizes**:
```sql
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) as table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) -
                   pg_relation_size(schemaname||'.'||tablename)) as index_size
FROM pg_tables
WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
LIMIT 20;
```

**Growth Forecast**:
```sql
SELECT
    resource_type,
    current_usage_percent,
    days_until_80_percent,
    days_until_90_percent,
    days_until_full
FROM v$capacity_forecast;
```

### Capacity Expansion

**Add Storage Volume**:
```bash
# 1. Create and attach new volume (cloud/SAN specific)

# 2. Format filesystem
sudo mkfs.ext4 /dev/sdb

# 3. Create mount point
sudo mkdir -p /var/lib/rustydb/data2

# 4. Mount volume
sudo mount /dev/sdb /var/lib/rustydb/data2

# 5. Add to fstab
echo "/dev/sdb /var/lib/rustydb/data2 ext4 defaults,nofail 0 2" | sudo tee -a /etc/fstab

# 6. Set ownership
sudo chown -R rustydb:rustydb /var/lib/rustydb/data2

# 7. Create tablespace
rusty-db-cli --command "
CREATE TABLESPACE data2 LOCATION '/var/lib/rustydb/data2';
"

# 8. Move large tables
rusty-db-cli --command "ALTER TABLE large_table SET TABLESPACE data2;"
```

**Scale Up (Vertical)**:
```bash
# 1. Schedule maintenance window

# 2. Create backup
rusty-db-backup --type full --output /backups/pre-scale.backup

# 3. Stop database
sudo systemctl stop rustydb

# 4. Resize VM/hardware (cloud-specific)

# 5. Update configuration
# Edit conf/rustydb.toml:
# buffer_pool_pages = 524288  # 2 GB (doubled)
# max_connections = 1000       # Increased

# 6. Start database
sudo systemctl start rustydb

# 7. Verify resources
rusty-db-cli --command "SELECT * FROM v$resource_usage;"
```

---

**Document Maintained By**: Enterprise Documentation Agent 4
**RustyDB Version**: 0.6.0
