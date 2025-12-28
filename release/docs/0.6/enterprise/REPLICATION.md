# RustyDB v0.6 Replication Strategies

**Version**: 0.6.0
**Last Updated**: December 2025
**Target Audience**: Database Administrators, Solutions Architects

---

## Table of Contents

1. [Overview](#overview)
2. [Replication Modes](#replication-modes)
3. [Physical Replication](#physical-replication)
4. [Logical Replication](#logical-replication)
5. [Multi-Master Replication](#multi-master-replication)
6. [Geo-Replication](#geo-replication)
7. [Conflict Resolution](#conflict-resolution)
8. [Configuration Guide](#configuration-guide)
9. [Monitoring](#monitoring)
10. [Best Practices](#best-practices)

---

## Overview

RustyDB v0.6 provides comprehensive replication capabilities for high availability, disaster recovery, read scaling, and data distribution across geographic regions.

### Replication Types

| Type | Use Case | Consistency | Performance | Complexity |
|------|----------|-------------|-------------|------------|
| **Synchronous** | Zero data loss | Strong | Lower throughput | Low |
| **Asynchronous** | High performance | Eventual | Highest throughput | Low |
| **Semi-Synchronous** | Balanced | Strong (1+ replica) | Medium | Medium |
| **Logical** | Selective replication | Eventual | Medium | Medium |
| **Multi-Master** | Active-active | Eventual (CRDT) | High | High |

---

## Replication Modes

### 1. Synchronous Replication

**Guarantee**: Zero data loss (RPO = 0)

**How it Works**:
```
Client → Primary → Write WAL → Replicate to Standby → Wait for ACK → Commit → ACK Client
```

**Configuration**:
```sql
ALTER SYSTEM SET replication_mode = 'synchronous';
ALTER SYSTEM SET synchronous_standby_names = 'standby1,standby2';
ALTER SYSTEM SET synchronous_commit = 'on';
```

**Trade-offs**:
- ✅ Zero data loss
- ✅ Strong consistency
- ❌ Higher latency
- ❌ Reduced throughput

### 2. Asynchronous Replication

**Guarantee**: Minimal data loss (RPO: seconds to minutes)

**How it Works**:
```
Client → Primary → Write WAL → Commit → ACK Client → Replicate to Standby (background)
```

**Configuration**:
```sql
ALTER SYSTEM SET replication_mode = 'asynchronous';
ALTER SYSTEM SET wal_sender_delay_ms = 100;  -- Batch WAL for efficiency
```

**Trade-offs**:
- ✅ Highest performance
- ✅ No latency impact on commits
- ❌ Potential data loss on primary failure
- ✅ Good for read replicas

### 3. Semi-Synchronous Replication

**Guarantee**: At least one standby has received data

**How it Works**:
```
Primary → Replicate to ALL standbys in parallel
       → Wait for FIRST ACK (from any standby)
       → Commit
```

**Configuration**:
```sql
ALTER SYSTEM SET replication_mode = 'semi_synchronous';
ALTER SYSTEM SET synchronous_standby_names = 'standby1,standby2';
ALTER SYSTEM SET quorum_based_replication = true;
```

**Trade-offs**:
- ✅ Balance between safety and performance
- ✅ Tolerates slow standbys
- ⚠️ Minimal data loss (one standby has data)

---

## Physical Replication

### WAL Streaming

**Architecture**:
```
┌─────────────┐          WAL Stream          ┌─────────────┐
│   Primary   │─────────────────────────────▶│   Standby   │
│             │  (continuous binary stream)   │ (recovery)  │
└─────────────┘                               └─────────────┘
```

**Setup**:
```bash
# On Primary
# Enable WAL archiving
ALTER SYSTEM SET wal_level = 'replica';
ALTER SYSTEM SET max_wal_senders = 10;
ALTER SYSTEM SET wal_keep_size = '10GB';

# Create replication slot
SELECT * FROM pg_create_physical_replication_slot('standby1_slot');

# On Standby
# Create standby.conf
cat > standby.conf <<EOF
primary_conninfo = 'host=primary port=5432 user=replication password=secret'
primary_slot_name = 'standby1_slot'
hot_standby = on
EOF

# Start standby in recovery mode
rustydb-server --standby --config=standby.conf
```

### Snapshot Replication

**Use Case**: Initial synchronization of new standby

```sql
-- On Primary
SELECT * FROM pg_start_backup('snapshot_backup', false, false);

-- Copy data files
rsync -avz /var/lib/rustydb/data/ standby:/var/lib/rustydb/data/

-- On Primary
SELECT * FROM pg_stop_backup();

-- On Standby
-- Create recovery.signal file
touch /var/lib/rustydb/data/recovery.signal

-- Start standby
rustydb-server --config=/etc/rustydb/rustydb.conf
```

---

## Logical Replication

### Publications and Subscriptions

**Concept**: Replicate individual tables with row filtering and column masking

**Create Publication** (on source):
```sql
-- Replicate all tables
CREATE PUBLICATION pub_all FOR ALL TABLES;

-- Replicate specific tables
CREATE PUBLICATION pub_sales
  FOR TABLE sales, customers, orders;

-- Replicate with filter
CREATE PUBLICATION pub_active_users
  FOR TABLE users
  WHERE (status = 'active');

-- Column masking
CREATE PUBLICATION pub_users_masked
  FOR TABLE users (id, name, email);  -- Exclude sensitive columns
```

**Create Subscription** (on target):
```sql
-- Full subscription
CREATE SUBSCRIPTION sub_all
  CONNECTION 'host=source.example.com port=5432 dbname=prod user=repl'
  PUBLICATION pub_all;

-- With replication slot
CREATE SUBSCRIPTION sub_sales
  CONNECTION 'host=source.example.com port=5432 dbname=prod'
  PUBLICATION pub_sales
  WITH (create_slot = true, slot_name = 'sub_sales_slot');

-- Disable initially (sync manually later)
CREATE SUBSCRIPTION sub_delayed
  CONNECTION 'host=source.example.com port=5432 dbname=prod'
  PUBLICATION pub_all
  WITH (enabled = false);
```

**Manage Subscriptions**:
```sql
-- Enable/disable
ALTER SUBSCRIPTION sub_all ENABLE;
ALTER SUBSCRIPTION sub_all DISABLE;

-- Refresh schema
ALTER SUBSCRIPTION sub_all REFRESH PUBLICATION;

-- Drop
DROP SUBSCRIPTION sub_all;
```

### Use Cases

**Selective Replication**:
- Replicate subset of tables to reporting database
- Cross-database replication
- Tenant-specific replication

**Data Transformation**:
- Change table structure on target
- Apply different indexes
- Different partitioning schemes

**Upgrade Path**:
- Logical replication between different RustyDB versions
- Zero-downtime upgrades

---

## Multi-Master Replication

### Architecture

```
┌──────────────┐     Bi-directional      ┌──────────────┐
│   Master 1   │◀─────Replication───────▶│   Master 2   │
│  (US West)   │                          │  (US East)   │
└──────────────┘                          └──────────────┘
       │                                         │
       │              ┌──────────────┐          │
       └──────────────│   Master 3   │──────────┘
                      │   (EU West)  │
                      └──────────────┘
```

### Configuration

**Create Replication Group**:
```sql
-- On all nodes
CREATE REPLICATION GROUP production_multimaster
  WITH (
    conflict_resolution = 'crdt',
    quorum_commit = true,
    min_quorum_size = 2
  );

-- Add sites
ALTER REPLICATION GROUP production_multimaster
  ADD SITE 'us_west' AT 'master1.example.com:5432';

ALTER REPLICATION GROUP production_multimaster
  ADD SITE 'us_east' AT 'master2.example.com:5432';

ALTER REPLICATION GROUP production_multimaster
  ADD SITE 'eu_west' AT 'master3.example.com:5432';

-- Add tables to replication
ALTER REPLICATION GROUP production_multimaster
  REPLICATE TABLE users, orders, products;
```

**Conflict Resolution Strategies**:

1. **CRDT (Conflict-free Replicated Data Types)**:
   ```sql
   ALTER TABLE users SET conflict_resolution = 'crdt_lww';  -- Last-Write-Wins
   ALTER TABLE counters SET conflict_resolution = 'crdt_counter';  -- Convergent counter
   ```

2. **Timestamp-Based (Last-Write-Wins)**:
   ```sql
   ALTER TABLE orders SET conflict_resolution = 'timestamp';
   ```

3. **Application-Defined**:
   ```sql
   CREATE FUNCTION resolve_user_conflict(old_row users, new_row users)
   RETURNS users AS $$
   BEGIN
     -- Custom logic
     IF new_row.priority > old_row.priority THEN
       RETURN new_row;
     ELSE
       RETURN old_row;
     END IF;
   END;
   $$ LANGUAGE plpgsql;

   ALTER TABLE users SET conflict_resolver = 'resolve_user_conflict';
   ```

### Convergence Monitoring

```sql
-- Check convergence status
SELECT site_id,
       last_sync_time,
       pending_transactions,
       divergence_count,
       convergence_score
FROM system.multimaster_convergence;

-- Alert on divergence
CREATE ALERT multimaster_divergence
  WHEN divergence_count > 100
  NOTIFY 'ops-team@example.com';
```

---

## Geo-Replication

### Multi-Region Setup

**Topology**:
```
Primary Region (US-WEST)        Standby Region (US-EAST)
┌─────────────────┐             ┌─────────────────┐
│  Primary DB     │────Async───▶│  Standby DB     │
│  + 2 Replicas   │  Repl       │  + 2 Replicas   │
└─────────────────┘             └─────────────────┘
        │                                │
        │      DR Region (EU-WEST)       │
        │      ┌─────────────────┐       │
        └─────▶│  DR Standby     │◀──────┘
               │  (Read-Only)    │
               └─────────────────┘
```

**Configuration**:
```sql
-- Define regions
CREATE REGION us_west
  WITH PRIMARY 'primary-usw.example.com:5432'
       STANDBYS ('standby1-usw.example.com:5432',
                 'standby2-usw.example.com:5432');

CREATE REGION us_east
  WITH PRIMARY 'primary-use.example.com:5432'
       STANDBYS ('standby1-use.example.com:5432',
                 'standby2-use.example.com:5432');

CREATE REGION eu_west
  WITH PRIMARY 'primary-euw.example.com:5432';

-- Set up inter-region replication
ALTER REGION us_west
  REPLICATE TO us_east WITH (mode = 'async', compression = true);

ALTER REGION us_west
  REPLICATE TO eu_west WITH (mode = 'async', priority = 'low');
```

**Failover Policy**:
```sql
ALTER SYSTEM SET region_failover_policy = 'nearest_healthy';
ALTER SYSTEM SET region_failover_priority = 'us_west,us_east,eu_west';
ALTER SYSTEM SET cross_region_failover_delay = 60;  -- seconds
```

---

## Conflict Resolution

### Conflict Types

**Update-Update Conflict**:
```
Time T1: US site updates user.email = 'new1@example.com'
Time T2: EU site updates user.email = 'new2@example.com'
Conflict: Which email wins?
```

**Delete-Update Conflict**:
```
Time T1: US site deletes user (id=123)
Time T2: EU site updates user.name = 'NewName' (id=123)
Conflict: Resurrection or stay deleted?
```

**Insert-Insert Conflict**:
```
Time T1: US site inserts order (id=1001, ...)
Time T2: EU site inserts order (id=1001, ...)
Conflict: Duplicate key
```

### CRDT Conflict Resolution

**LWW (Last-Write-Wins)**:
```sql
CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  name TEXT,
  email TEXT,
  updated_at TIMESTAMP WITH TIME ZONE,
  site_id TEXT
) WITH (conflict_resolution = 'lww');

-- Conflict resolution:
-- Compare updated_at, keep newer
-- If same timestamp, compare site_id (deterministic)
```

**Counter CRDT**:
```sql
CREATE TABLE page_views (
  page_id INTEGER PRIMARY KEY,
  view_count BIGINT CRDT_COUNTER
);

-- Updates are commutative
-- Site 1: view_count += 10
-- Site 2: view_count += 5
-- Result: view_count = original + 10 + 5 (guaranteed)
```

### Manual Conflict Resolution

```sql
-- View conflicts
SELECT * FROM system.replication_conflicts
WHERE resolved = false;

-- Resolve conflict (pick winner)
SELECT resolve_conflict(
  conflict_id => 1234,
  resolution => 'use_local'  -- or 'use_remote', 'merge'
);

-- Merge conflict (custom logic)
SELECT resolve_conflict(
  conflict_id => 1235,
  resolution => 'custom',
  merged_value => '{"name": "Best Name", "email": "best@example.com"}'::jsonb
);
```

---

## Configuration Guide

### Replication Slots

**Create**:
```sql
-- Physical slot
SELECT * FROM pg_create_physical_replication_slot('standby1_slot');

-- Logical slot
SELECT * FROM pg_create_logical_replication_slot('logical_slot', 'pgoutput');

-- List slots
SELECT * FROM pg_replication_slots;
```

**Delete**:
```sql
SELECT pg_drop_replication_slot('slot_name');
```

### Replication Monitoring

```sql
-- Replication lag
SELECT client_hostname,
       state,
       sync_state,
       replay_lag_seconds,
       write_lag_seconds
FROM system.replication_status;

-- WAL status
SELECT wal_position,
       wal_sent,
       wal_received,
       wal_replayed,
       (wal_sent - wal_replayed) AS lag_bytes
FROM system.wal_replication_status;
```

---

## Monitoring

### Key Metrics

```sql
-- Replication lag (seconds)
SELECT standby_name,
       extract(epoch from (now() - replay_time)) AS lag_seconds
FROM system.replication_lag;

-- Replication throughput
SELECT standby_name,
       bytes_per_second,
       transactions_per_second
FROM system.replication_throughput;

-- Conflict rate
SELECT site_id,
       conflicts_per_hour,
       avg_resolution_time_ms
FROM system.multimaster_conflicts;
```

### Alerting

```sql
-- High lag alert
CREATE ALERT replication_lag
  WHEN lag_seconds > 60
  NOTIFY 'oncall@example.com';

-- Replication stopped
CREATE ALERT replication_stopped
  WHEN state != 'streaming'
  FOR 300 SECONDS  -- 5 minutes
  NOTIFY 'critical@example.com';
```

---

## Best Practices

1. **Use Replication Slots**: Prevent WAL deletion
2. **Monitor Lag**: Set alerts for lag > 5 seconds
3. **Test Failover**: Monthly failover drills
4. **Size WAL Properly**: `wal_keep_size` = 2x daily WAL volume
5. **Use Compression**: For cross-region replication
6. **Logical Replication**: For version upgrades
7. **Multi-Master**: Only when necessary (complexity)
8. **CRDT**: Preferred for multi-master conflict resolution
9. **Network Bandwidth**: Ensure sufficient bandwidth for replication
10. **Backups**: Replicas are NOT backups (implement separate backup strategy)

---

**See Also**:
- [Clustering](./CLUSTERING.md)
- [RAC](./RAC.md)
- [Backup and Recovery](../operations/BACKUP_RECOVERY.md)

**Document Version**: 1.0
**Last Updated**: December 2025
