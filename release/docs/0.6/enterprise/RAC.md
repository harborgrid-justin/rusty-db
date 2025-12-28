# RustyDB v0.6 Real Application Clusters (RAC)

**Version**: 0.6.0
**Last Updated**: December 2025
**Target Audience**: Enterprise Database Architects, Senior DBAs
**Complexity**: Advanced

---

## Table of Contents

1. [Introduction](#introduction)
2. [RAC Architecture](#rac-architecture)
3. [Cache Fusion](#cache-fusion)
4. [Global Resource Directory (GRD)](#global-resource-directory-grd)
5. [Cluster Interconnect](#cluster-interconnect)
6. [Instance Recovery](#instance-recovery)
7. [Parallel Query Execution](#parallel-query-execution)
8. [Installation and Configuration](#installation-and-configuration)
9. [Performance Tuning](#performance-tuning)
10. [Monitoring](#monitoring)
11. [Troubleshooting](#troubleshooting)

---

## Introduction

RustyDB Real Application Clusters (RAC) is an Oracle RAC-compatible clustering technology that enables multiple database instances to access a single shared database simultaneously. RAC provides both high availability and horizontal scalability for mission-critical workloads.

### Key Benefits

**High Availability**:
- No single point of failure
- Automatic instance recovery (<30 seconds)
- Zero-downtime maintenance
- Online rolling upgrades

**Horizontal Scalability**:
- Add capacity by adding nodes
- Linear scalability for OLTP workloads
- Parallel query execution across instances
- Load distribution across all nodes

**Operational Efficiency**:
- Consolidate multiple databases
- Pay-as-you-grow model
- Simplified management
- Resource pooling

### Use Cases

| Use Case | Description | Benefits |
|----------|-------------|----------|
| **E-Commerce** | High-traffic online stores | Handle peak loads, zero downtime |
| **Financial Trading** | Real-time trading platforms | Ultra-low latency, HA |
| **SaaS Platforms** | Multi-tenant applications | Elastic scaling, tenant isolation |
| **ERP Systems** | Oracle EBS, SAP | Application compatibility, HA |
| **Data Warehouses** | OLAP workloads | Parallel query, fast analytics |

### Comparison with Standard Clustering

| Feature | Standard Cluster | RAC |
|---------|-----------------|-----|
| Active Nodes | 1 (leader) | All nodes |
| Write Scalability | No | Yes |
| Failover Time | 30-60s | <10s |
| Cache Coherency | None | Cache Fusion |
| Resource Coordination | None | GRD |
| Parallel Query | Single node | Cross-instance |

---

## RAC Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    RustyDB RAC Cluster                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐   │
│  │  Instance 1    │  │  Instance 2    │  │  Instance 3    │   │
│  ├────────────────┤  ├────────────────┤  ├────────────────┤   │
│  │ Buffer Cache   │  │ Buffer Cache   │  │ Buffer Cache   │   │
│  │ Shared Pool    │  │ Shared Pool    │  │ Shared Pool    │   │
│  │ Lock Manager   │  │ Lock Manager   │  │ Lock Manager   │   │
│  └────────┬───────┘  └────────┬───────┘  └────────┬───────┘   │
│           │                   │                   │            │
│           └───────────────────┼───────────────────┘            │
│                               │                                │
│              ┌────────────────▼────────────────┐               │
│              │  Cluster Interconnect (GCS/GES) │               │
│              │  - Cache Fusion Protocol        │               │
│              │  - Global Resource Directory    │               │
│              │  - Message Passing              │               │
│              └────────────────┬────────────────┘               │
│                               │                                │
│              ┌────────────────▼────────────────┐               │
│              │      Shared Storage             │               │
│              │  - Data Files                   │               │
│              │  - Control Files                │               │
│              │  - Redo Logs                    │               │
│              │  - OCR (Cluster Registry)       │               │
│              └─────────────────────────────────┘               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Components

**Instances**:
- Each instance runs on a separate server
- Has its own SGA (System Global Area)
- Shares access to database files
- Coordinates via cluster interconnect

**Shared Storage**:
- Must be accessible by all instances
- Supports: SAN, NAS, clustered filesystems
- Required components:
  - Data files (.dbf)
  - Control files
  - Redo logs (per-instance)
  - Archive logs
  - OCR (Oracle Cluster Registry equivalent)

**Cluster Interconnect**:
- High-speed private network
- UDP-based messaging
- Recommended: 10Gbps or higher
- Redundancy required (bonded interfaces)

**Global Services**:
- **GCS (Global Cache Service)**: Cache Fusion coordination
- **GES (Global Enqueue Service)**: Lock coordination
- **GRD (Global Resource Directory)**: Resource mastering

---

## Cache Fusion

### Overview

Cache Fusion is the heart of RAC technology. It allows instances to transfer database blocks directly from memory to memory, without disk I/O. This enables true active-active clustering with minimal performance overhead.

### How Cache Fusion Works

**Traditional Shared-Disk (No Cache Fusion)**:
```
Instance 1 needs Block 100 (currently in Instance 2's cache)
    │
    ▼
Write block to disk (Instance 2)
    │
    ▼
Read block from disk (Instance 1)
    │
Total I/O: 2 disk operations
```

**With Cache Fusion**:
```
Instance 1 needs Block 100 (currently in Instance 2's cache)
    │
    ▼
Transfer block over interconnect (Instance 2 → Instance 1)
    │
Total I/O: 0 disk operations, 1 network message
```

### Block Transfer Process

```
┌──────────────┐                               ┌──────────────┐
│  Instance 1  │                               │  Instance 2  │
│              │                               │              │
│ Need Block   │────1. Request Block 100──────▶│ Has Block in │
│ 100          │                               │ Cache (CR)   │
│              │                               │              │
│              │◀───2. Send Block via GCS──────│              │
│              │    (Interconnect)             │              │
│ Block        │                               │ Mark as PI   │
│ Received     │                               │ (Past Image) │
│              │                               │              │
│ Mark as      │                               │              │
│ Current      │                               │              │
└──────────────┘                               └──────────────┘
```

**Block Modes**:
- **NULL (N)**: Block not in cache
- **SHARED (S)**: Read-only copy, multiple instances can hold
- **EXCLUSIVE (X)**: Write access, only one instance holds
- **CURRENT (CR)**: Most recent version
- **PAST IMAGE (PI)**: Older version kept for consistency

### Cache Fusion Modes

**CR (Consistent Read) Block Transfer**:
- For SELECT queries
- Multiple instances can have CR copies
- Non-blocking transfers

**Current Block Transfer**:
- For DML operations (INSERT, UPDATE, DELETE)
- Transfers write authority
- Requires lock coordination with GES

**Light-Weight Cache Transfer**:
- Optimized for no modifications
- Direct memory copy
- No disk involvement

### Global Cache Service (GCS)

**Responsibilities**:
- Track block ownership across instances
- Coordinate block transfers
- Maintain cache coherency
- Handle block conflicts

**GCS Messages**:
- CR request: Request consistent read version
- Current request: Request write authority
- Grant message: Transfer ownership
- Invalidate: Mark cached block as stale

### Performance Characteristics

**Typical Latencies**:
- Local buffer access: ~10 µs
- Cache Fusion transfer: ~500 µs (10Gbps interconnect)
- Disk read: ~10,000 µs (SSD), ~5,000,000 µs (HDD)

**Cache Fusion is 20x faster than SSD, 10,000x faster than HDD**

### Configuration

```sql
-- Enable Cache Fusion
ALTER SYSTEM SET cache_fusion_enabled = true;

-- Set interconnect for Cache Fusion
ALTER SYSTEM SET cluster_interconnects = '192.168.100.0/24';

-- Configure GCS parameters
ALTER SYSTEM SET gcs_server_processes = 4;  -- Per instance
ALTER SYSTEM SET gcs_shadow_processes = 2;

-- Buffer cache sizing (per instance)
ALTER SYSTEM SET buffer_cache_size = '32GB';

-- Enable Cache Fusion statistics
ALTER SYSTEM SET cache_fusion_stats = true;
```

---

## Global Resource Directory (GRD)

### Overview

The Global Resource Directory is a distributed data structure that tracks the location and status of all resources (blocks, locks, etc.) in the RAC cluster. Each resource has a master instance responsible for coordinating access.

### Resource Mastering

**Hash-Based Distribution**:
```
Resource ID → Hash Function → Master Instance

Example:
Block 12345 → Hash(12345) mod 3 → Instance 2 (master)
```

**Resource Roles**:
- **Master**: Coordinates access, grants locks
- **Holder**: Currently has the resource
- **Waiter**: Waiting for the resource

### Dynamic Remastering

When access patterns change, GRD can dynamically reassign resource masters to optimize performance.

**Remastering Triggers**:
- Hot block detection (>100 accesses/sec from non-master instance)
- Instance addition/removal
- Manual intervention

**Remastering Process**:
```
1. Detect Hot Block
   Instance 1 accessing Block 100 (mastered by Instance 3) frequently
         │
         ▼
2. Initiate Remaster
   Instance 1 requests remastering from Instance 3
         │
         ▼
3. Freeze Resource
   Instance 3 freezes all operations on Block 100
         │
         ▼
4. Transfer Master Info
   Instance 3 → Instance 1: resource state, holders, waiters
         │
         ▼
5. Activate New Master
   Instance 1 becomes master of Block 100
         │
         ▼
6. Resume Operations
   Normal operations resume with new master
```

**Configuration**:
```sql
-- Enable dynamic remastering
ALTER SYSTEM SET dynamic_remastering = true;

-- Remastering threshold (accesses per second)
ALTER SYSTEM SET remaster_threshold = 100;

-- Automatic remastering interval
ALTER SYSTEM SET remaster_interval = 300;  -- seconds

-- View resource masters
SELECT block_id, master_instance, access_count
FROM system.grd_resource_masters
ORDER BY access_count DESC
LIMIT 100;
```

### Global Enqueue Service (GES)

**Enqueue Types**:
- **TX (Transaction)**: Row-level locks
- **TM (Table)**: Table-level locks
- **TA (Transaction Abort)**: Transaction coordination
- **SQ (Sequence)**: Sequence number allocation

**Lock Modes**:
- **NULL (N)**: No access
- **SUB-SHARE (SS)**: Intent share
- **SUB-EXCLUSIVE (SX)**: Intent exclusive
- **SHARE (S)**: Read access
- **EXCLUSIVE (X)**: Write access

**Deadlock Detection**:
- Global deadlock detection across instances
- Timeout-based (default: 60 seconds)
- Victim selection by lowest cost

---

## Cluster Interconnect

### Network Requirements

**Bandwidth**:
- Minimum: 1Gbps
- Recommended: 10Gbps
- High-performance: 40Gbps / 100Gbps InfiniBand

**Latency**:
- Target: <100 µs
- Acceptable: <500 µs
- Unacceptable: >1ms (consider network upgrade)

**Redundancy**:
- Minimum: 2 bonded interfaces
- Recommended: 4 interfaces in bonded configuration
- Failover time: <1 second

### Network Configuration

```bash
# Configure bonded interface (Linux)
cat > /etc/sysconfig/network-scripts/ifcfg-bond0 <<EOF
DEVICE=bond0
TYPE=Bond
BONDING_MASTER=yes
IPADDR=192.168.100.10
NETMASK=255.255.255.0
BONDING_OPTS="mode=802.3ad miimon=100"
BOOTPROTO=none
ONBOOT=yes
EOF

# Add slave interfaces
for iface in eth2 eth3; do
  cat > /etc/sysconfig/network-scripts/ifcfg-$iface <<EOF
DEVICE=$iface
TYPE=Ethernet
MASTER=bond0
SLAVE=yes
BOOTPROTO=none
ONBOOT=yes
EOF
done

# Restart networking
systemctl restart network
```

**RustyDB Configuration**:
```sql
-- Primary interconnect
ALTER SYSTEM SET cluster_interconnects = '192.168.100.0/24';

-- Secondary interconnect (failover)
ALTER SYSTEM SET cluster_interconnects_secondary = '192.168.101.0/24';

-- Message aggregation (reduce message count)
ALTER SYSTEM SET interconnect_aggregation = true;

-- Compression (reduce bandwidth, increase CPU)
ALTER SYSTEM SET interconnect_compression = false;  -- Disable for low latency
```

### Heartbeat Monitoring

**Purpose**: Detect node failures

**Configuration**:
```sql
ALTER SYSTEM SET heartbeat_interval_ms = 1000;  -- 1 second
ALTER SYSTEM SET heartbeat_timeout_ms = 5000;   -- 5 seconds
ALTER SYSTEM SET heartbeat_miss_threshold = 3;  -- 3 missed = failure
```

**Split-Brain Prevention**:
```sql
-- Require quorum for operations
ALTER SYSTEM SET split_brain_protection = true;
ALTER SYSTEM SET min_quorum_size = 2;  -- (N/2 + 1)

-- Fencing method
ALTER SYSTEM SET fencing_method = 'disk_based';  -- or 'network_based'
```

---

## Instance Recovery

### Automatic Instance Recovery

When an instance fails, surviving instances automatically recover the failed instance's uncommitted transactions.

**Recovery Process**:

```
1. Failure Detection (5 seconds)
   ├─ Heartbeat timeout
   ├─ Interconnect failure
   └─ Health check failure
         │
         ▼
2. Recovery Initiation
   ├─ Leader assigns recovery to surviving instance
   ├─ Failed instance's redo logs identified
   └─ Lock resources reconfigured
         │
         ▼
3. Redo Application (parallel)
   ├─ Read redo logs from shared storage
   ├─ Apply changes to data blocks
   └─ Roll forward committed transactions
         │
         ▼
4. Undo Application
   ├─ Identify incomplete transactions
   ├─ Roll back uncommitted changes
   └─ Release locks
         │
         ▼
5. Resource Remastering
   ├─ Redistribute failed instance's resources
   ├─ Update GRD metadata
   └─ Resume normal operations
         │
         ▼
6. Recovery Complete (typically <30 seconds)
```

**Recovery Time Factors**:
- Redo log size
- Number of uncommitted transactions
- Redo apply parallelism
- Storage performance

**Configuration**:
```sql
-- Parallel recovery processes
ALTER SYSTEM SET recovery_parallelism = 8;

-- Redo buffer sizing
ALTER SYSTEM SET redo_buffer_size = '128MB';

-- Fast recovery area
ALTER SYSTEM SET fast_recovery_area = '/rac/fra';
ALTER SYSTEM SET fast_recovery_area_size = '100GB';

-- Monitor recovery
SELECT instance_id, recovery_status, recovery_progress
FROM system.instance_recovery;
```

### Manual Recovery

```sql
-- Force recovery of specific instance
ALTER SYSTEM RECOVER INSTANCE 2;

-- Verify recovery status
SELECT * FROM system.instance_recovery_status;
```

---

## Parallel Query Execution

### Cross-Instance Parallelism

RAC enables parallel query execution across multiple instances, dramatically improving performance for analytical queries.

**Architecture**:
```
                     Query Coordinator (Instance 1)
                              │
                Parse → Optimize → Generate Plan
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
    Instance 1            Instance 2            Instance 3
        │                     │                     │
   Scan Part 1           Scan Part 2           Scan Part 3
   (10M rows)            (10M rows)            (10M rows)
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
                      Merge Results (30M rows)
                              │
                         Return to Client
```

**Query Plan**:
```sql
EXPLAIN SELECT sum(amount) FROM sales;

-- Output:
QUERY PLAN
─────────────────────────────────────────────────────
Finalize Aggregate (cost=1000..1001 rows=1)
  ->  Gather (cost=900..950 rows=3)
        Workers: 3 (across 3 instances)
        ->  Partial Aggregate (cost=800..900 rows=10M)
              ->  Parallel Seq Scan on sales (cost=0..800 rows=10M)
                    Instance: 1,2,3
```

**Configuration**:
```sql
-- Enable cross-instance parallelism
ALTER SYSTEM SET parallel_instance_group = 'ALL';

-- Maximum parallel workers per query
ALTER SYSTEM SET max_parallel_workers_per_query = 16;

-- Parallel workers per instance
ALTER SYSTEM SET parallel_workers_per_instance = 8;

-- Degree of parallelism (automatic)
ALTER SYSTEM SET parallel_degree_policy = 'AUTO';

-- Execute query with hint
SELECT /*+ PARALLEL(sales, 12) */ sum(amount) FROM sales;
```

**Performance**:
- 2-node RAC: ~1.8x speedup
- 3-node RAC: ~2.5x speedup
- 4-node RAC: ~3.2x speedup
- Diminishing returns beyond 8 nodes (coordination overhead)

---

## Installation and Configuration

### Prerequisites

**Hardware**:
- 2-8 servers (physical or virtual)
- 64GB+ RAM per server
- 10Gbps+ interconnect network
- Shared storage (SAN/NAS/Clustered FS)

**Software**:
- RustyDB 0.6 Enterprise Edition
- Linux (RHEL/CentOS 8+, Ubuntu 20.04+)
- Clustered filesystem (OCFS2, GFS2) or raw devices

### Installation Steps

**1. Install RustyDB on All Nodes**:
```bash
# On each node
sudo yum install rustydb-enterprise-0.6.x86_64.rpm

# Verify
rustydb --version
```

**2. Configure Shared Storage**:
```bash
# Format shared volume (on one node only)
sudo mkfs.ocfs2 -L "racdata" /dev/mapper/racvg-raclv

# Mount on all nodes
sudo mount -t ocfs2 /dev/mapper/racvg-raclv /rac/data

# Add to /etc/fstab
echo "/dev/mapper/racvg-raclv /rac/data ocfs2 _netdev,defaults 0 0" | sudo tee -a /etc/fstab
```

**3. Configure Interconnect**:
```bash
# On all nodes - see Network Configuration section
```

**4. Initialize RAC Cluster**:
```bash
# On node1 (primary)
sudo rustydb-admin rac init \
  --cluster-name production \
  --data-directory /rac/data \
  --node-id node1 \
  --interconnect 192.168.100.10

# Join from other nodes
# On node2
sudo rustydb-admin rac join \
  --cluster-name production \
  --node-id node2 \
  --interconnect 192.168.100.11 \
  --primary-node node1:5433

# On node3
sudo rustydb-admin rac join \
  --cluster-name production \
  --node-id node3 \
  --interconnect 192.168.100.12 \
  --primary-node node1:5433
```

**5. Create Database**:
```bash
# On node1
sudo rustydb-admin rac create-database \
  --database production_db \
  --instances 3
```

**6. Start All Instances**:
```bash
# On all nodes
sudo systemctl start rustydb-rac
sudo systemctl enable rustydb-rac

# Verify
sudo rustydb-admin rac status
```

---

## Performance Tuning

### Buffer Cache Sizing

**Rule of Thumb**: Total buffer cache across all instances = 70-80% of total RAM

**Example** (4 nodes, 64GB RAM each):
```sql
-- Per instance: 0.75 * 64GB = 48GB
ALTER SYSTEM SET buffer_cache_size = '48GB' SCOPE=ALL;

-- Total cluster cache: 4 * 48GB = 192GB
```

### GCS/GES Tuning

```sql
-- Increase GCS processes for high concurrency
ALTER SYSTEM SET gcs_server_processes = 8 SCOPE=ALL;

-- Reduce messaging overhead
ALTER SYSTEM SET interconnect_aggregation = true;

-- Batch GCS operations
ALTER SYSTEM SET gcs_batch_size = 100;
```

### Redo Log Optimization

```sql
-- Increase redo buffer per instance
ALTER SYSTEM SET redo_buffer_size = '256MB';

-- Multiple redo threads
ALTER SYSTEM SET redo_threads = 4;

-- Fast redo writes
ALTER SYSTEM SET redo_write_optimization = true;
```

### Application Design

**Best Practices**:
- Partition tables across instances
- Use sequences with CACHE 1000 or higher
- Minimize cross-instance transactions
- Use connection pooling with instance affinity

**Anti-Patterns**:
- SELECT FOR UPDATE across instances (hot blocks)
- Singleton tables (single master)
- Excessive locking

---

## Monitoring

### Key Metrics

```sql
-- Cache Fusion statistics
SELECT instance_id,
       cache_transfers_current,
       cache_transfers_cr,
       gc_blocks_transferred,
       gc_avg_transfer_time_ms
FROM system.rac_cache_fusion_stats;

-- GCS performance
SELECT message_type,
       total_messages,
       avg_latency_us,
       p95_latency_us,
       p99_latency_us
FROM system.rac_gcs_stats;

-- Instance recovery status
SELECT instance_id,
       status,
       recovery_progress_pct,
       estimated_completion_time
FROM system.rac_recovery_status;

-- Interconnect bandwidth
SELECT node_id,
       bytes_sent_per_sec,
       bytes_received_per_sec,
       message_rate_per_sec,
       errors
FROM system.rac_interconnect_stats;
```

### Dashboards

**Grafana Example**:
```
- Cache Fusion Transfer Rate
- GCS Message Latency (p95, p99)
- Interconnect Bandwidth Utilization
- Instance Health Score
- Active Parallel Queries
```

---

## Troubleshooting

### High Cache Transfer Latency

**Symptom**: gc_avg_transfer_time_ms > 5ms

**Causes**:
- Network congestion
- Slow interconnect
- Excessive transfers

**Solutions**:
```sql
-- Check interconnect health
SELECT * FROM system.rac_interconnect_stats;

-- Identify hot blocks
SELECT block_id, transfer_count, master_instance
FROM system.rac_hot_blocks
ORDER BY transfer_count DESC
LIMIT 100;

-- Remaster hot blocks
ALTER SYSTEM REMASTER BLOCK 12345 TO INSTANCE 2;
```

### Frequent Instance Evictions

**Symptom**: Instance marked as failed but actually healthy

**Causes**:
- Network flapping
- Heartbeat timeouts
- Resource starvation

**Solutions**:
```sql
-- Increase heartbeat timeout
ALTER SYSTEM SET heartbeat_timeout_ms = 10000;

-- Check system resources
SELECT * FROM system.resource_usage WHERE instance_id = 'node2';

-- Review interconnect errors
SELECT * FROM system.network_errors WHERE interface = 'bond0';
```

---

**See Also**:
- [Clustering Documentation](./CLUSTERING.md)
- [Performance Tuning Guide](../operations/PERFORMANCE_TUNING.md)
- [RAC Test Report](/docs/RAC_TEST_REPORT.md)

**Document Version**: 1.0
**Last Updated**: December 2025
