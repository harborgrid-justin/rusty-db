# RustyDB v0.6.0 - Troubleshooting Guide

**Document Version**: 1.0
**Release**: v0.6.0
**Last Updated**: 2025-12-28
**Classification**: Enterprise Operations

---

## Table of Contents

1. [Troubleshooting Approach](#troubleshooting-approach)
2. [Startup Issues](#startup-issues)
3. [Connection Issues](#connection-issues)
4. [Performance Issues](#performance-issues)
5. [Replication Issues](#replication-issues)
6. [Data Issues](#data-issues)
7. [Resource Issues](#resource-issues)
8. [Security Issues](#security-issues)
9. [Diagnostic Tools](#diagnostic-tools)
10. [Getting Support](#getting-support)

---

## Troubleshooting Approach

### General Troubleshooting Steps

**1. Gather Information**:
- What changed recently?
- When did the issue start?
- Is it affecting all users or specific ones?
- Can you reproduce the issue?

**2. Check Logs**:
```bash
# System logs
sudo journalctl -u rustydb -n 200 --no-pager

# Application logs
sudo tail -200 /var/lib/rustydb/instances/default/logs/rustydb.log

# Error logs only
sudo journalctl -u rustydb -p err --since "1 hour ago"
```

**3. Check System Health**:
```bash
# Service status
sudo systemctl status rustydb

# Database health
curl http://localhost:8080/api/v1/admin/health | jq

# Resource usage
rusty-db-cli --command "SELECT * FROM v$resource_usage;"
```

**4. Check Connectivity**:
```bash
# Port listening
sudo netstat -tlnp | grep rusty-db

# Test connection
rusty-db-cli --command "SELECT 1;"
```

**5. Review Recent Changes**:
```bash
# Recent configuration changes
git log -10 --oneline conf/rustydb.toml

# Recent upgrades
rusty-db-cli --command "SELECT * FROM v$upgrade_history;"
```

---

## Startup Issues

### Issue: Service Fails to Start

**Symptoms**:
```bash
sudo systemctl status rustydb
● rustydb.service - RustyDB Database Server
     Loaded: loaded
     Active: failed (Result: exit-code)
```

**Diagnosis**:
```bash
# Check detailed logs
sudo journalctl -u rustydb -n 100 --no-pager

# Check for common issues
sudo ls -la /var/lib/rustydb/instances/default/
sudo lsof -i :5432
```

**Common Causes & Solutions**:

**1. Port Already in Use**:
```bash
# Find what's using the port
sudo lsof -i :5432
sudo ss -tlnp | grep 5432

# Solution 1: Stop conflicting service
sudo systemctl stop postgresql

# Solution 2: Change RustyDB port
# Edit conf/rustydb.toml:
# [server]
# listen_port = 5433

sudo systemctl restart rustydb
```

**2. Permission Denied**:
```bash
# Check ownership
sudo ls -la /var/lib/rustydb/instances/default/

# Fix ownership
sudo chown -R rustydb:rustydb /var/lib/rustydb/
sudo chown -R rustydb:rustydb /var/log/rustydb/

# Fix permissions
sudo chmod 750 /var/lib/rustydb/instances/default/data
sudo chmod 640 /var/lib/rustydb/instances/default/conf/rustydb.toml

sudo systemctl restart rustydb
```

**3. Configuration Error**:
```bash
# Validate configuration
rusty-db-server --check-config /var/lib/rustydb/instances/default/conf/rustydb.toml

# Check for syntax errors
grep -n 'ERROR\|error' /var/lib/rustydb/instances/default/logs/rustydb.log | tail -20

# Restore from backup
sudo cp /var/lib/rustydb/instances/default/conf/rustydb.toml.backup \
       /var/lib/rustydb/instances/default/conf/rustydb.toml

sudo systemctl restart rustydb
```

**4. Corrupted Data Directory**:
```bash
# Check for corruption indicators
sudo journalctl -u rustydb | grep -i corrupt

# Solution: Restore from backup
sudo systemctl stop rustydb

rusty-db-restore --input /backups/latest.backup \
  --data-dir /var/lib/rustydb/instances/default/data

sudo systemctl start rustydb
```

**5. Insufficient Disk Space**:
```bash
# Check disk space
df -h /var/lib/rustydb

# Solution: Free up space
sudo find /var/lib/rustydb/instances/default/logs -name "*.log.*" -mtime +7 -delete
sudo find /var/lib/rustydb/instances/default/cache -type f -delete

# Or expand disk
# (See MAINTENANCE.md for capacity expansion)
```

### Issue: Crashes on Startup

**Symptoms**:
```
Service starts then immediately exits
```

**Diagnosis**:
```bash
# Run in foreground to see errors
sudo -u rustydb /opt/rustydb/current/bin/rusty-db-server \
  --home /var/lib/rustydb/instances/default \
  --foreground

# Check core dumps
ls -la /var/lib/rustydb/instances/default/diag/
```

**Solutions**:

**1. Binary Corruption**:
```bash
# Verify binary integrity
sha256sum /opt/rustydb/current/bin/rusty-db-server

# Reinstall from package
sudo cp /backups/rusty-db-server /opt/rustydb/current/bin/
sudo chmod +x /opt/rustydb/current/bin/rusty-db-server
```

**2. Incompatible Data Format**:
```bash
# Check data format version
cat /var/lib/rustydb/instances/default/data/meta/data-format-version

# Check binary version
rusty-db-server --version

# Solution: Restore data from backup compatible with binary version
```

---

## Connection Issues

### Issue: Cannot Connect to Database

**Symptoms**:
```
Error: Connection refused
Error: Connection timeout
```

**Diagnosis**:
```bash
# 1. Check service is running
sudo systemctl status rustydb

# 2. Check port is listening
sudo netstat -tlnp | grep 5432

# 3. Test local connection
rusty-db-cli --command "SELECT 1;"

# 4. Test from application server
telnet database.company.com 5432
```

**Solutions**:

**1. Service Not Running**:
```bash
sudo systemctl start rustydb
sudo systemctl enable rustydb
```

**2. Firewall Blocking**:
```bash
# Check firewall rules
sudo ufw status
sudo iptables -L -n | grep 5432

# Add firewall rule (Ubuntu)
sudo ufw allow from 10.0.1.0/24 to any port 5432 proto tcp

# Add firewall rule (RHEL)
sudo firewall-cmd --permanent --add-rich-rule='
  rule family="ipv4"
  source address="10.0.1.0/24"
  port port="5432" protocol="tcp"
  accept'
sudo firewall-cmd --reload
```

**3. Wrong Listen Address**:
```bash
# Check current configuration
grep listen_host /var/lib/rustydb/instances/default/conf/rustydb.toml

# Should be:
# listen_host = "0.0.0.0"  # For network access
# or
# listen_host = "10.0.1.100"  # Specific interface

# Update and restart
sudo systemctl restart rustydb
```

**4. Network Issues**:
```bash
# Check network connectivity
ping database.company.com

# Check DNS resolution
nslookup database.company.com

# Check routing
traceroute database.company.com

# Test port connectivity
nc -zv database.company.com 5432
```

### Issue: Connection Pool Exhausted

**Symptoms**:
```
Error: Too many connections
Error: Connection pool timeout
```

**Diagnosis**:
```sql
-- Check current connections
SELECT COUNT(*) FROM v$active_sessions;

-- Check max connections
SELECT setting FROM pg_settings WHERE name = 'max_connections';

-- Check connection pool status
SELECT
    pool_name,
    max_connections,
    active_connections,
    idle_connections,
    waiting_connections
FROM v$connection_pools;
```

**Solutions**:

**1. Increase Max Connections**:
```bash
# Edit conf/rustydb.toml
[server]
max_connections = 1000  # Increased from 500

# Restart service
sudo systemctl restart rustydb
```

**2. Kill Idle Connections**:
```sql
-- Find long-running idle connections
SELECT
    session_id,
    username,
    idle_time_seconds,
    query
FROM v$active_sessions
WHERE status = 'idle'
AND idle_time_seconds > 3600
ORDER BY idle_time_seconds DESC;

-- Kill specific session
SELECT pg_terminate_backend(session_id);
```

**3. Implement Connection Pooling** (application side):
```typescript
// Use Node.js adapter with connection pooling
import { createRustyDbClient, createConfig } from '@rustydb/adapter';

const config = createConfig()
  .server({ host: 'localhost', port: 5432 })
  .pool({
    maxConnections: 50,
    minConnections: 10,
    connectionTimeout: 30000
  })
  .build();

const client = await createRustyDbClient(config);
```

---

## Performance Issues

### Issue: Slow Queries

**Symptoms**:
```
Query takes > 1 second
Application timeout
```

**Diagnosis**:
```sql
-- Find slow queries
SELECT
    query_id,
    query_text,
    total_time,
    calls,
    mean_time,
    max_time
FROM v$pg_stat_statements
WHERE mean_time > 100  -- ms
ORDER BY total_time DESC
LIMIT 10;

-- Analyze specific query
EXPLAIN ANALYZE
SELECT * FROM customers WHERE email = 'user@example.com';
```

**Solutions**:

**1. Missing Index**:
```sql
-- Find missing indexes
SELECT
    table_name,
    column_names,
    estimated_improvement
FROM v$missing_indexes
ORDER BY estimated_improvement DESC;

-- Create index
CREATE INDEX CONCURRENTLY idx_customers_email ON customers(email);
```

**2. Outdated Statistics**:
```sql
-- Update statistics
ANALYZE customers;
ANALYZE ALL;
```

**3. Inefficient Query**:
```sql
-- Rewrite query with better conditions
-- Before:
SELECT * FROM orders WHERE YEAR(created_at) = 2025;

-- After (can use index on created_at):
SELECT * FROM orders
WHERE created_at >= '2025-01-01'
AND created_at < '2026-01-01';
```

**4. Query Timeout Too Low**:
```bash
# Edit conf/rustydb.toml
[server]
query_timeout_ms = 300000  # 5 minutes

sudo systemctl reload rustydb
```

### Issue: High CPU Usage

**Symptoms**:
```
CPU usage > 90%
Slow query response
```

**Diagnosis**:
```bash
# Check CPU usage
top -u rustydb

# Check active queries
rusty-db-cli --command "
SELECT
    session_id,
    query,
    cpu_time_ms,
    duration_seconds
FROM v$active_sessions
ORDER BY cpu_time_ms DESC;
"
```

**Solutions**:

**1. Terminate Expensive Queries**:
```sql
-- Kill specific session
SELECT pg_terminate_backend(session_id);
```

**2. Add Query Limits**:
```sql
-- Set per-session CPU limit
ALTER USER expensive_user SET cpu_time_limit = 60000;  -- 1 minute
```

**3. Optimize Queries**:
```sql
-- Use LIMIT for large result sets
SELECT * FROM large_table ORDER BY created_at DESC LIMIT 100;

-- Use indexes
CREATE INDEX CONCURRENTLY idx_large_table_created_at ON large_table(created_at DESC);
```

### Issue: High Memory Usage

**Symptoms**:
```
Memory usage > 90%
OOM killer invoked
```

**Diagnosis**:
```bash
# Check memory usage
free -h
top -u rustydb

# Check buffer pool usage
rusty-db-cli --command "SELECT * FROM v$buffer_pool_stats;"
```

**Solutions**:

**1. Reduce Buffer Pool Size**:
```bash
# Edit conf/rustydb.toml
[storage]
buffer_pool_pages = 65536  # Reduced from 262144 (1 GB to 256 MB)

sudo systemctl restart rustydb
```

**2. Disable Query Cache Temporarily**:
```bash
# Edit conf/rustydb.toml
[cache]
query_cache_enabled = false

sudo systemctl reload rustydb
```

**3. Add More RAM**:
```bash
# Scale up VM/hardware
# See MAINTENANCE.md for capacity expansion
```

---

## Replication Issues

### Issue: Replication Lag

**Symptoms**:
```
Standby falling behind primary
Data inconsistency between nodes
```

**Diagnosis**:
```sql
-- Check replication lag
SELECT
    standby_name,
    lag_seconds,
    lag_bytes,
    last_wal_receive_time,
    last_wal_replay_time
FROM v$replication_lag
ORDER BY lag_seconds DESC;

-- Check replication status
SELECT * FROM v$replication_status;
```

**Solutions**:

**1. Network Issues**:
```bash
# Test network between primary and standby
ping -c 10 standby.company.com

# Check network bandwidth
iperf3 -c standby.company.com

# Check for packet loss
mtr standby.company.com
```

**2. Standby Overloaded**:
```bash
# On standby, check resource usage
top
iostat -x 1

# Solution: Scale up standby resources
# Or: Reduce query load on standby
```

**3. WAL Archiving Issues**:
```bash
# Check WAL archive status
rusty-db-cli --command "SELECT * FROM v$archive_status;"

# Check archive directory space
df -h /var/lib/rustydb/archive

# Clean up old archives
rusty-db-cli --command "SELECT pg_archive_cleanup();"
```

### Issue: Standby Not Replicating

**Symptoms**:
```
Standby status: disconnected
No replication activity
```

**Diagnosis**:
```bash
# On primary
rusty-db-cli --command "SELECT * FROM v$replication_status;"

# On standby
sudo journalctl -u rustydb -n 100 | grep replication

# Check connectivity
psql -h primary.company.com -p 5432 -U replicator -c "SELECT 1;"
```

**Solutions**:

**1. Authentication Issues**:
```bash
# Check pg_hba.conf on primary
grep replication /etc/rustydb/pg_hba.conf

# Should have entry like:
# host replication replicator 10.0.2.0/24 scram-sha-256

# Test authentication
psql -h primary.company.com -p 5432 -U replicator replication=database
```

**2. Recovery Configuration**:
```bash
# On standby, check recovery.conf
cat /var/lib/rustydb/instances/default/data/recovery.conf

# Should have:
# standby_mode = on
# primary_conninfo = 'host=primary port=5432 user=replicator password=...'
# restore_command = 'cp /archive/wal/%f %p'

sudo systemctl restart rustydb
```

---

## Data Issues

### Issue: Data Corruption

**Symptoms**:
```
Error: checksum mismatch
Error: invalid page header
```

**Diagnosis**:
```bash
# Check for corruption in logs
sudo journalctl -u rustydb | grep -i corrupt

# Run consistency check
rusty-db-cli --command "SELECT * FROM v$data_integrity_check;"
```

**Solutions**:

**1. Restore from Backup**:
```bash
sudo systemctl stop rustydb

rusty-db-restore --input /backups/latest.backup \
  --data-dir /var/lib/rustydb/instances/default/data \
  --verify

sudo systemctl start rustydb
```

**2. Point-in-Time Recovery** (if corruption is recent):
```bash
# Restore to before corruption
# See BACKUP_RECOVERY.md for PITR procedures
```

### Issue: Missing Data

**Symptoms**:
```
Expected rows not found
Count doesn't match
```

**Diagnosis**:
```sql
-- Check transaction log
SELECT
    transaction_id,
    username,
    operation,
    table_name,
    timestamp
FROM v$transaction_log
WHERE table_name = 'customers'
AND operation = 'DELETE'
ORDER BY timestamp DESC
LIMIT 100;

-- Check audit log
SELECT * FROM v$audit_log
WHERE object_name = 'customers'
ORDER BY timestamp DESC
LIMIT 100;
```

**Solutions**:

**1. Point-in-Time Recovery** (before deletion):
```bash
# Identify deletion time
# Restore to just before deletion
# See BACKUP_RECOVERY.md
```

**2. Table-Level Restore**:
```bash
# Restore specific table from backup
# See BACKUP_RECOVERY.md for table-level restore
```

---

## Resource Issues

### Issue: Disk Full

**Symptoms**:
```
Error: No space left on device
Write operations failing
```

**Diagnosis**:
```bash
# Check disk usage
df -h /var/lib/rustydb

# Find large files
sudo du -h /var/lib/rustydb/instances/default/ | sort -rh | head -20

# Check table sizes
rusty-db-cli --command "
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
FROM pg_tables
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
LIMIT 10;
"
```

**Solutions**:

**1. Clean Up Logs**:
```bash
# Remove old logs
sudo find /var/lib/rustydb/instances/default/logs -name "*.log.*" -mtime +7 -delete

# Clean cache
sudo rm -rf /var/lib/rustydb/instances/default/cache/*
```

**2. Vacuum Tables**:
```sql
VACUUM FULL;
```

**3. Archive Old Data**:
```sql
-- Move old data to archive table
CREATE TABLE orders_archive AS
SELECT * FROM orders WHERE created_at < '2024-01-01';

DELETE FROM orders WHERE created_at < '2024-01-01';
VACUUM orders;
```

**4. Expand Disk**:
```bash
# See MAINTENANCE.md for capacity expansion
```

---

## Security Issues

### Issue: Failed Login Attempts

**Symptoms**:
```
Multiple failed login attempts
Account lockouts
```

**Diagnosis**:
```sql
-- Check failed logins
SELECT
    username,
    ip_address,
    COUNT(*) as attempts,
    MAX(timestamp) as last_attempt
FROM v$failed_logins
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY username, ip_address
ORDER BY attempts DESC;
```

**Solutions**:

**1. Block Malicious IPs**:
```bash
# Add firewall rule to block IP
sudo ufw deny from 192.168.1.100

# Or using iptables
sudo iptables -A INPUT -s 192.168.1.100 -j DROP
```

**2. Unlock Account**:
```sql
-- Unlock user account
ALTER USER username UNLOCK ACCOUNT;
```

**3. Strengthen Security**:
```bash
# Edit conf/rustydb.toml
[auth]
max_failed_attempts = 3  # Stricter
lockout_duration_ms = 900000  # 15 minutes

sudo systemctl reload rustydb
```

---

## Diagnostic Tools

### Generate Diagnostic Bundle

```bash
rusty-db-diag --output /tmp/rustydb-diag-$(date +%Y%m%d_%H%M%S).tar.gz
```

**Bundle Contents**:
- Configuration files
- Recent logs (last 1000 lines)
- System information
- Resource usage statistics
- Active sessions
- Recent queries
- Replication status (if applicable)

### Enable Debug Logging

```bash
# Edit conf/rustydb.toml
[logging]
level = "debug"

sudo systemctl reload rustydb
```

**Note**: Debug logging impacts performance. Disable after troubleshooting.

### Performance Profiling

```bash
# Enable query profiling
rusty-db-cli --command "SET enable_profiling = true;"

# Run query
rusty-db-cli --command "SELECT * FROM slow_query;"

# View profile
rusty-db-cli --command "SELECT * FROM v$query_profile ORDER BY total_time DESC;"
```

---

## Getting Support

### Before Contacting Support

**Gather Information**:
1. RustyDB version: `rusty-db-cli --command "SELECT version();"`
2. Operating system: `uname -a`
3. Configuration file: `/var/lib/rustydb/instances/default/conf/rustydb.toml`
4. Recent logs: Last 500 lines from logs
5. Error messages: Exact error text
6. Recent changes: Configuration, upgrades, schema changes
7. Diagnostic bundle: `rusty-db-diag --output /tmp/diag.tar.gz`

### Support Channels

**Community Support**:
- GitHub Issues: https://github.com/rustydb/rusty-db/issues
- Community Forum: https://community.rustydb.io
- Documentation: https://docs.rustydb.io

**Enterprise Support**:
- Email: support@rustydb.io
- Phone: +1-XXX-XXX-XXXX (24/7 for critical issues)
- Portal: https://support.rustydb.io

### Issue Severity Levels

**Critical (P1)**:
- Production database down
- Data corruption
- Security breach
- Response: 15 minutes

**High (P2)**:
- Severe performance degradation
- Replication failure
- Major feature not working
- Response: 2 hours

**Medium (P3)**:
- Moderate performance issues
- Non-critical feature issues
- Response: 1 business day

**Low (P4)**:
- Questions
- Documentation requests
- Enhancement requests
- Response: 3 business days

---

## Common Error Messages

### Error: "Connection refused"

**Cause**: Database service not running or firewall blocking

**Solution**:
```bash
sudo systemctl start rustydb
sudo ufw allow 5432/tcp
```

### Error: "Too many connections"

**Cause**: Connection pool exhausted

**Solution**:
```bash
# Increase max_connections in conf/rustydb.toml
# Or kill idle connections
rusty-db-cli --command "SELECT pg_terminate_backend(session_id) FROM v$active_sessions WHERE status = 'idle';"
```

### Error: "No space left on device"

**Cause**: Disk full

**Solution**:
```bash
# Clean up logs and cache
sudo rm -rf /var/lib/rustydb/instances/default/cache/*
# Run VACUUM
rusty-db-cli --command "VACUUM FULL;"
```

### Error: "Permission denied"

**Cause**: Incorrect file permissions

**Solution**:
```bash
sudo chown -R rustydb:rustydb /var/lib/rustydb/
sudo chmod 750 /var/lib/rustydb/instances/default/data
```

### Error: "Checksum mismatch"

**Cause**: Data corruption

**Solution**:
```bash
# Restore from backup
rusty-db-restore --input /backups/latest.backup \
  --data-dir /var/lib/rustydb/instances/default/data
```

---

**Document Maintained By**: Enterprise Documentation Agent 4
**RustyDB Version**: 0.6.0
**Last Updated**: 2025-12-28
