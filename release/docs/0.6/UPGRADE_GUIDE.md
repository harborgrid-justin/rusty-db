# RustyDB v0.6.0 Upgrade Guide

This guide provides comprehensive instructions for upgrading to RustyDB v0.6.0 from previous versions.

## Overview

RustyDB v0.6.0 maintains **full backward compatibility** with v0.5.x releases. The upgrade process is straightforward with no mandatory data migration or configuration changes.

## Quick Upgrade Path

### For Development Environments

```bash
# 1. Stop current server
pkill rusty-db-server

# 2. Backup current data (optional but recommended)
cp -r ./data ./data.backup.$(date +%Y%m%d)

# 3. Replace binary
cp builds/linux/rusty-db-server /path/to/rusty-db-server

# 4. Start new version
./rusty-db-server
```

### For Production Environments

See [Detailed Upgrade Procedure](#detailed-upgrade-procedure-production) below.

## Compatibility Matrix

| Component | v0.5.x | v0.6.0 | Compatible | Notes |
|-----------|--------|--------|------------|-------|
| Binary | ✓ | ✓ | ✅ Yes | Direct replacement |
| Configuration | ✓ | ✓ | ✅ Yes | Old config supported |
| Data Format | ✓ | ✓ | ✅ Yes | No migration needed |
| WAL Format | ✓ | ✓ | ✅ Yes | Compatible |
| API Endpoints | ✓ | ✓ | ✅ Yes | All existing endpoints work |
| GraphQL Schema | ✓ | ✓ | ✅ Yes | Backward compatible |
| Node.js Adapter | v0.2.x | v0.6.0 | ⚠️ Upgrade | See Node.js section |

## What's New in v0.6.0

### New Features
- 54 new REST API endpoints
- 24 new GraphQL operations
- Node.js Adapter v0.6.0 with native bindings
- Performance optimizations (+50-65% TPS)
- Enhanced security features
- Complete API coverage

### Improvements
- Better query performance (+20-30%)
- Improved buffer pool efficiency (+15% hit rate)
- Faster transaction processing
- Enhanced documentation

## Pre-Upgrade Checklist

### Required Steps

- [ ] Review release notes ([RELEASE_NOTES.md](./RELEASE_NOTES.md))
- [ ] Review changelog ([CHANGELOG.md](./CHANGELOG.md))
- [ ] Verify system requirements
- [ ] Plan maintenance window (if production)
- [ ] Backup current data directory
- [ ] Backup current WAL directory
- [ ] Document current configuration
- [ ] Test upgrade in staging environment

### System Requirements

**Minimum**:
- Linux kernel 3.2.0+
- 4 GB RAM
- 20 GB disk space
- x86-64 processor

**Recommended**:
- Linux kernel 4.0+
- 8 GB RAM
- 100 GB SSD
- x86-64 with AVX2 support

**Verification**:
```bash
# Check kernel version
uname -r

# Check RAM
free -h

# Check disk space
df -h

# Check CPU features
lscpu | grep -i avx
```

## Detailed Upgrade Procedure (Production)

### Phase 1: Preparation (30 minutes)

#### Step 1.1: Backup Current System

```bash
# Create backup directory
BACKUP_DIR=/backup/rustydb-$(date +%Y%m%d-%H%M%S)
mkdir -p $BACKUP_DIR

# Backup data directory
cp -r /var/lib/rusty-db/data $BACKUP_DIR/

# Backup WAL directory
cp -r /var/lib/rusty-db/wal $BACKUP_DIR/

# Backup configuration
cp /etc/rusty-db/rustydb.toml $BACKUP_DIR/

# Backup binaries
cp /usr/local/bin/rusty-db-server $BACKUP_DIR/rusty-db-server.old
cp /usr/local/bin/rusty-db-cli $BACKUP_DIR/rusty-db-cli.old

# Verify backup
ls -lh $BACKUP_DIR/
```

#### Step 1.2: Document Current State

```bash
# Record current version
/usr/local/bin/rusty-db-server --version > $BACKUP_DIR/version.txt

# Record current configuration
cat /etc/rusty-db/rustydb.toml > $BACKUP_DIR/config.txt

# Record current processes
ps aux | grep rusty-db > $BACKUP_DIR/processes.txt

# Record current connections
netstat -tulpn | grep rusty > $BACKUP_DIR/ports.txt
```

#### Step 1.3: Health Check

```bash
# Check server health
curl http://localhost:8080/api/v1/health

# Check database status
./rusty-db-cli --command "SELECT version();"

# Check disk space
df -h /var/lib/rusty-db

# Check memory usage
free -h
```

### Phase 2: Server Shutdown (5 minutes)

#### Step 2.1: Graceful Shutdown

```bash
# Stop accepting new connections (if supported)
# curl -X POST http://localhost:8080/api/v1/admin/disable-connections

# Wait for active transactions to complete
# Monitor: curl http://localhost:8080/api/v1/monitoring/active-transactions

# Stop systemd service
sudo systemctl stop rustydb

# Verify server stopped
ps aux | grep rusty-db-server
# Should show no processes

# Verify ports released
netstat -tulpn | grep 5432
netstat -tulpn | grep 8080
# Should show no listeners
```

#### Step 2.2: Verify Clean Shutdown

```bash
# Check last WAL entries
ls -lh /var/lib/rusty-db/wal/

# Check for temp files
find /var/lib/rusty-db -name "*.tmp"

# Check logs for errors
journalctl -u rustydb -n 50
```

### Phase 3: Binary Upgrade (5 minutes)

#### Step 3.1: Install New Binaries

```bash
# Download v0.6.0 binaries (or use pre-built from builds/)
cd /path/to/rusty-db

# Verify binary integrity
sha256sum builds/linux/rusty-db-server
# Compare with official checksum

# Install server binary
sudo cp builds/linux/rusty-db-server /usr/local/bin/
sudo chmod 755 /usr/local/bin/rusty-db-server

# Install CLI binary
sudo cp builds/linux/rusty-db-cli /usr/local/bin/
sudo chmod 755 /usr/local/bin/rusty-db-cli

# Verify installation
/usr/local/bin/rusty-db-server --version
# Should show: RustyDB v0.6.0

/usr/local/bin/rusty-db-cli --version
# Should show: RustyDB CLI v0.6.0
```

#### Step 3.2: Verify Dependencies

```bash
# Check binary dependencies
ldd /usr/local/bin/rusty-db-server

# Verify all dependencies are present
# Should not show "not found" for any library
```

### Phase 4: Configuration Update (Optional, 10 minutes)

#### Step 4.1: Review Configuration

```bash
# View current configuration
cat /etc/rusty-db/rustydb.toml

# Compare with v0.6.0 defaults
cat /path/to/rusty-db/conf/rustydb.toml
```

#### Step 4.2: Update Configuration (if needed)

**No mandatory changes required**, but you may want to enable new features:

```toml
# /etc/rusty-db/rustydb.toml

# Existing configuration remains valid
[database]
data_dir = "/var/lib/rusty-db/data"
wal_dir = "/var/lib/rusty-db/wal"
page_size = 4096
buffer_pool_size = 1000

# Optional: Enable new v0.6.0 features
[performance]
# Enhanced buffer pool
enable_arc_enhanced = true
enable_prefetch_enhanced = true
enable_dirty_page_flusher = true

# Lock-free page table
enable_lock_free_page_table = true
page_table_shards = 64

# WAL optimizations
enable_striped_wal = true
wal_stripe_count = 8

[optimizer]
# Hardware-aware cost model
enable_hardware_calibration = true

# Adaptive execution
enable_adaptive_execution = true
max_parallel_degree = 32

# Plan baselines
enable_plan_baselines = true
```

### Phase 5: Server Restart (5 minutes)

#### Step 5.1: Start New Version

```bash
# Start systemd service
sudo systemctl start rustydb

# Check status
sudo systemctl status rustydb

# Verify startup in logs
journalctl -u rustydb -f
# Look for: "RustyDB is ready to accept connections"
```

#### Step 5.2: Verify Server Health

```bash
# Test PostgreSQL protocol
nc -zv localhost 5432
# Expected: Connection succeeded

# Test REST API
curl http://localhost:8080/api/v1/health
# Expected: {"status":"healthy","version":"0.6.0",...}

# Test GraphQL
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ health { status version } }"}'
# Expected: GraphQL response with version 0.6.0

# Test WebSocket
# Using websocat: websocat ws://localhost:8080/ws
```

### Phase 6: Validation (15 minutes)

#### Step 6.1: Functional Testing

```bash
# Test database operations
./rusty-db-cli << 'EOF'
-- Check version
SELECT version();

-- Test table creation
CREATE TABLE test_upgrade (id INT, name TEXT);

-- Test insert
INSERT INTO test_upgrade VALUES (1, 'test');

-- Test select
SELECT * FROM test_upgrade;

-- Test update
UPDATE test_upgrade SET name = 'updated' WHERE id = 1;

-- Test delete
DELETE FROM test_upgrade WHERE id = 1;

-- Cleanup
DROP TABLE test_upgrade;
EOF
```

#### Step 6.2: API Testing

```bash
# Test new REST endpoints
curl http://localhost:8080/api/v1/security/encryption/status

# Test new GraphQL operations
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ encryption_status { enabled tablespaces } }"}'

# Test streaming
curl http://localhost:8080/api/v1/streams/topics
```

#### Step 6.3: Performance Validation

```bash
# Run simple benchmark
time ./rusty-db-cli --command "SELECT COUNT(*) FROM large_table;"

# Check transaction throughput
curl http://localhost:8080/api/v1/monitoring/metrics | grep tps

# Check buffer pool hit rate
curl http://localhost:8080/api/v1/monitoring/buffer-pool | grep hit_rate
# Should see improvement over v0.5.x
```

### Phase 7: Monitoring (Ongoing)

#### Step 7.1: Set Up Monitoring

```bash
# Monitor logs
journalctl -u rustydb -f

# Monitor metrics
watch -n 5 'curl -s http://localhost:8080/api/v1/monitoring/metrics'

# Monitor connections
watch -n 5 'netstat -an | grep :5432 | grep ESTABLISHED | wc -l'
```

#### Step 7.2: Performance Comparison

```bash
# Compare TPS before/after
# Before upgrade: ~10,000 TPS
# After v0.6.0: ~16,500 TPS (expected +65%)

# Compare query performance
# Run same queries, compare execution times
# Expected improvement: +20-30%

# Compare buffer pool hit rate
# Before: ~82%
# After: ~95% (expected +15%)
```

## Upgrading Node.js Adapter

### From v0.2.x to v0.6.0

The Node.js adapter has been significantly enhanced in v0.6.0.

#### Step 1: Update Package

```bash
cd /path/to/your/app

# Update package.json
npm install @rustydb/adapter@0.6.0

# Or update in package.json
{
  "dependencies": {
    "@rustydb/adapter": "^0.6.0"
  }
}

npm install
```

#### Step 2: Update Code

**Old Code (v0.2.x)**:
```javascript
const { createRustyDbClient } = require('@rustydb/adapter');

const client = await createRustyDbClient({
  host: 'localhost',
  port: 5432
});

const result = await client.query('SELECT * FROM users');
```

**New Code (v0.6.0)** - Basic usage remains compatible:
```javascript
const { createRustyDbClient, createConfig } = require('@rustydb/adapter');

// Old API still works
const client = await createRustyDbClient({
  host: 'localhost',
  port: 5432
});

// OR: Use new config builder API
const config = createConfig()
  .server({ host: 'localhost', port: 5432 })
  .api({ baseUrl: 'http://localhost:8080' })
  .build();

const client = await createRustyDbClient(config);
await client.initialize();

// Use new features
const result = await client.query('SELECT * FROM users');
```

#### Step 3: Leverage New Features (Optional)

**Prepared Statements**:
```javascript
// Create prepared statement
const stmt = await client.prepare('SELECT * FROM users WHERE id = ?');

// Execute with parameters
const result = await stmt.execute([42]);

// Close when done
await stmt.close();
```

**Result Streaming**:
```javascript
// Stream large result sets
const stream = await client.streamQuery('SELECT * FROM large_table');

stream.on('row', (row) => {
  console.log('Row:', row);
});

stream.on('end', () => {
  console.log('Stream complete');
});

// Or use async iterator
for await (const row of stream) {
  console.log('Row:', row);
}
```

**Connection Pooling**:
```javascript
import { ConnectionPool } from '@rustydb/adapter';

const pool = new ConnectionPool({
  min: 2,
  max: 10,
  idleTimeout: 30000,
  healthCheckInterval: 10000
});

await pool.initialize();

// Acquire connection
const conn = await pool.acquire();

// Use connection
await conn.query('SELECT 1');

// Release back to pool
await pool.release(conn);

// Get statistics
const stats = pool.getStats();
console.log('Active connections:', stats.active);
```

#### Step 4: Native Bindings (Optional)

For maximum performance, build native bindings:

```bash
cd /path/to/rusty-db/nodejs-adapter

# Build native module
npm run build:native

# This creates native bindings that the adapter will automatically use
# Falls back to HTTP if native bindings unavailable
```

## Configuration Migration

### Old Config (Deprecated but Supported)

```rust
// Still works in v0.6.0
use rusty_db::Config;

let config = Config {
    data_dir: "./data".to_string(),
    page_size: 4096,
    buffer_pool_size: 1000,
    port: 5432,
};
```

### New Config (Recommended)

```rust
// Recommended for v0.6.0+
use rusty_db::common::DatabaseConfig;

let config = DatabaseConfig {
    data_dir: "./data".to_string(),
    wal_dir: "./wal".to_string(),
    page_size: 4096,
    buffer_pool_size: 1000,
    port: 5432,
    api_port: 8080,
    enable_rest_api: true,
};
```

## Data Migration

**No data migration required** for v0.6.0 upgrade. Data formats are compatible.

### Verification

```bash
# Start server with existing data
./rusty-db-server

# Verify data integrity
./rusty-db-cli << 'EOF'
-- Check table count
SELECT COUNT(*) FROM information_schema.tables;

-- Verify row counts match pre-upgrade values
SELECT table_name, COUNT(*) FROM your_tables;
EOF
```

## Rollback Procedure

If you encounter issues with v0.6.0, you can rollback:

### Step 1: Stop v0.6.0 Server

```bash
sudo systemctl stop rustydb
```

### Step 2: Restore Old Binary

```bash
# Restore from backup
sudo cp $BACKUP_DIR/rusty-db-server.old /usr/local/bin/rusty-db-server
sudo cp $BACKUP_DIR/rusty-db-cli.old /usr/local/bin/rusty-db-cli

# Verify version
/usr/local/bin/rusty-db-server --version
# Should show v0.5.x
```

### Step 3: Restore Configuration (if changed)

```bash
sudo cp $BACKUP_DIR/config.txt /etc/rusty-db/rustydb.toml
```

### Step 4: Restart Old Version

```bash
sudo systemctl start rustydb
sudo systemctl status rustydb
```

### Step 5: Verify Rollback

```bash
curl http://localhost:8080/api/v1/health
./rusty-db-cli --command "SELECT version();"
```

## Troubleshooting

### Issue: Server fails to start

**Symptom**: Server doesn't start after upgrade

**Diagnosis**:
```bash
# Check logs
journalctl -u rustydb -n 50

# Check binary
ldd /usr/local/bin/rusty-db-server

# Check permissions
ls -la /var/lib/rusty-db/
```

**Solutions**:
1. Verify binary dependencies are met
2. Check data directory permissions: `sudo chown -R rustydb:rustydb /var/lib/rusty-db`
3. Ensure ports are available: `sudo netstat -tulpn | grep -E '(5432|8080)'`
4. Check disk space: `df -h /var/lib/rusty-db`

### Issue: Performance degradation

**Symptom**: Slower performance after upgrade

**Diagnosis**:
```bash
# Check buffer pool hit rate
curl http://localhost:8080/api/v1/monitoring/buffer-pool

# Check transaction metrics
curl http://localhost:8080/api/v1/monitoring/metrics
```

**Solutions**:
1. Enable performance optimizations in configuration
2. Increase buffer pool size
3. Check if new features need tuning
4. Review [Performance Tuning](#performance-tuning) section

### Issue: API compatibility issues

**Symptom**: Old API calls failing

**Diagnosis**:
```bash
# Test specific endpoint
curl -v http://localhost:8080/api/v1/old-endpoint
```

**Solutions**:
1. Verify endpoint path (all v0.5.x endpoints supported)
2. Check request format
3. Review API_REFERENCE.md for changes
4. Contact support if issue persists

## Performance Tuning

### Recommended Settings for v0.6.0

```toml
[database]
# Increase buffer pool for better performance
buffer_pool_size = 10000  # ~40 MB for 4KB pages

# Enable WAL optimizations
wal_stripe_count = 8
wal_target_latency_ms = 10.0

[performance]
# Enable all v0.6.0 optimizations
enable_arc_enhanced = true
enable_prefetch_enhanced = true
enable_dirty_page_flusher = true
enable_lock_free_page_table = true
enable_striped_wal = true

# Lock manager sharding
lock_manager_shards = 64

[optimizer]
# Hardware-aware optimization
enable_hardware_calibration = true
enable_adaptive_execution = true
enable_plan_baselines = true
max_parallel_degree = 32
```

### Monitoring Performance

```bash
# Monitor TPS
curl http://localhost:8080/api/v1/monitoring/metrics | jq '.transactions_per_second'

# Monitor buffer pool hit rate
curl http://localhost:8080/api/v1/monitoring/buffer-pool | jq '.hit_rate'

# Monitor query performance
curl http://localhost:8080/api/v1/monitoring/slow-queries
```

## Getting Help

### Documentation
- [Release Notes](./RELEASE_NOTES.md)
- [Known Issues](./KNOWN_ISSUES.md)
- [Deployment Guide](../../../docs/DEPLOYMENT_GUIDE.md)
- [Operations Guide](../../../docs/OPERATIONS_GUIDE.md)

### Support Channels
- GitHub Issues: Report bugs and issues
- Documentation: Comprehensive guides in /docs
- Community: User forums and discussions

## Summary

### Upgrade Checklist

- [ ] Backup completed
- [ ] Health check passed
- [ ] Server stopped cleanly
- [ ] New binaries installed
- [ ] Configuration updated (if needed)
- [ ] Server restarted successfully
- [ ] Functional tests passed
- [ ] API tests passed
- [ ] Performance validated
- [ ] Monitoring configured
- [ ] Node.js adapter updated (if applicable)

### Expected Results

After successful upgrade to v0.6.0:

- ✅ Server version: 0.6.0
- ✅ All existing data accessible
- ✅ All existing APIs functional
- ✅ New APIs available
- ✅ Performance improvements visible
- ✅ Zero downtime (for rolling upgrades)
- ✅ Backward compatible

### Success Criteria

- Server starts without errors
- Health check returns "healthy"
- All functional tests pass
- Performance meets or exceeds v0.5.x
- No data loss
- All applications work correctly

---

**Upgrade Guide Version**: 1.0
**For RustyDB**: v0.6.0
**Last Updated**: December 28, 2025
