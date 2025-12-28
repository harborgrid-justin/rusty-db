# RustyDB v0.6 Clustering and High Availability

**Version**: 0.6.0
**Last Updated**: December 2025
**Target Audience**: Database Administrators, DevOps Engineers, Solution Architects

---

## Table of Contents

1. [Overview](#overview)
2. [Clustering Architecture](#clustering-architecture)
3. [Consensus Protocol (Raft)](#consensus-protocol-raft)
4. [High Availability Modes](#high-availability-modes)
5. [Failover Mechanisms](#failover-mechanisms)
6. [Load Balancing](#load-balancing)
7. [Distributed Query Execution](#distributed-query-execution)
8. [Data Migration and Rebalancing](#data-migration-and-rebalancing)
9. [Monitoring and Health Checks](#monitoring-and-health-checks)
10. [Configuration Guide](#configuration-guide)
11. [Operational Procedures](#operational-procedures)
12. [Troubleshooting](#troubleshooting)

---

## Overview

RustyDB v0.6 provides enterprise-grade clustering capabilities for high availability, horizontal scalability, and fault tolerance. The clustering implementation supports multiple deployment topologies from traditional active-passive HA to sophisticated RAC-style active-active configurations.

### Key Features

- **Raft Consensus**: Proven distributed consensus algorithm for leader election and log replication
- **Automatic Failover**: Sub-minute failover with zero data loss
- **Split-Brain Prevention**: Quorum-based decisions prevent data corruption
- **Geo-Distribution**: Multi-region support with region-aware routing
- **Dynamic Membership**: Add/remove nodes without downtime
- **Load Balancing**: Intelligent query routing based on node health and load

### Use Cases

| Use Case | Recommended Topology | Nodes | Recovery Time |
|----------|---------------------|-------|---------------|
| Development/Testing | Single Node | 1 | N/A |
| Production (Small) | Active-Passive | 2 | <5 minutes |
| Production (Medium) | 3-Node Cluster | 3 | <1 minute |
| Production (Large) | RAC Cluster | 4-8 | <30 seconds |
| Global Distribution | Multi-Region | 8+ | <5 minutes |

---

## Clustering Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                    RustyDB Cluster                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
│  │   Node 1   │  │   Node 2   │  │   Node 3   │           │
│  │  (Leader)  │  │ (Follower) │  │ (Follower) │           │
│  └────────────┘  └────────────┘  └────────────┘           │
│         │               │               │                   │
│         └───────────────┼───────────────┘                   │
│                         │                                   │
│              ┌──────────▼──────────┐                        │
│              │   Raft Consensus    │                        │
│              │   - Leader Election │                        │
│              │   - Log Replication │                        │
│              │   - Membership      │                        │
│              └─────────────────────┘                        │
│                         │                                   │
│              ┌──────────▼──────────┐                        │
│              │ Distributed Services │                       │
│              │ - Query Execution   │                        │
│              │ - Data Migration    │                        │
│              │ - Load Balancer     │                        │
│              │ - Health Monitor    │                        │
│              └─────────────────────┘                        │
│                         │                                   │
│              ┌──────────▼──────────┐                        │
│              │    Shared/Local     │                        │
│              │      Storage        │                        │
│              └─────────────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

### Node Roles

**Leader**:
- Accepts write operations
- Coordinates distributed transactions
- Makes cluster-wide decisions
- Replicates logs to followers

**Follower**:
- Accepts read operations (optional)
- Replicates leader's log
- Participates in leader election
- Can be promoted to leader

**Observer** (optional):
- Read-only replica
- Does not participate in elections
- Useful for read scaling and reporting

### Communication

**Raft Protocol**:
- Port: 5433 (configurable)
- Protocol: TCP with TLS 1.3
- Heartbeat interval: 1 second
- Election timeout: 5 seconds

**Client Connections**:
- Port: 5432 (PostgreSQL wire protocol)
- Load balanced via connection pooler
- Automatic retry on node failure

---

## Consensus Protocol (Raft)

### Raft Overview

RustyDB uses the Raft consensus algorithm for distributed coordination. Raft ensures:
- **Strong consistency**: All nodes agree on the same log
- **Fault tolerance**: Cluster operates as long as majority is healthy
- **Leader election**: Automatic leader election on failure

### State Machine

```
┌─────────────┐
│  Follower   │
└──────┬──────┘
       │
       │ Timeout (no heartbeat)
       │
       ▼
┌─────────────┐
│  Candidate  │◄──────┐
└──────┬──────┘       │
       │              │ Split vote
       │ Win election │
       │              │
       ▼              │
┌─────────────┐       │
│   Leader    │───────┘
└─────────────┘

```

### Election Process

1. **Follower timeout**: No heartbeat from leader for 5 seconds
2. **Become candidate**: Increment term, vote for self
3. **Request votes**: Send RequestVote RPCs to all peers
4. **Collect votes**: Wait for majority (quorum)
5. **Become leader**: If quorum achieved, start sending heartbeats
6. **Retry**: If split vote, start new election

### Log Replication

```
Leader receives write → Append to local log → Send AppendEntries RPC
                                                      │
                                                      ▼
                                           ┌──────────────────┐
                                           │  Wait for quorum │
                                           └────────┬─────────┘
                                                    │
                                  ┌─────────────────┼─────────────────┐
                                  ▼                 ▼                 ▼
                            Follower 1        Follower 2        Follower 3
                            Append log        Append log        Append log
                            Send ACK          Send ACK          Send ACK
                                  │                 │                 │
                                  └─────────────────┼─────────────────┘
                                                    ▼
                                              ┌───────────┐
                                              │  Commit   │
                                              │  & Apply  │
                                              └───────────┘
```

**Guarantees**:
- Log entries are replicated to majority before commit
- Committed entries are never lost
- Logs remain consistent across nodes

### Configuration

```toml
[cluster.raft]
election_timeout_ms = 5000
heartbeat_interval_ms = 1000
snapshot_interval = 10000  # Log entries between snapshots
max_log_entries = 100000   # Trigger snapshot
retention_logs = 1000      # Keep after snapshot
```

---

## High Availability Modes

### 1. Active-Passive (Traditional HA)

**Architecture**:
```
┌─────────────────┐      Synchronous      ┌─────────────────┐
│   Primary       │─────Replication───────▶│   Standby       │
│   (Active)      │                        │   (Passive)     │
└─────────────────┘                        └─────────────────┘
```

**Characteristics**:
- Primary handles all traffic
- Standby ready for immediate promotion
- Synchronous replication (zero data loss)
- Failover time: <5 minutes

**Configuration**:
```sql
ALTER SYSTEM SET ha_mode = 'active_passive';
ALTER SYSTEM SET synchronous_standby_names = 'standby1';
ALTER SYSTEM SET failover_mode = 'automatic';
```

**Use Cases**:
- Traditional enterprise deployments
- Regulatory requirements for zero data loss
- Budget constraints (2 servers minimum)

### 2. Active-Active (Quorum-Based)

**Architecture**:
```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Node 1    │◄───▶│   Node 2    │◄───▶│   Node 3    │
│  (Leader)   │     │ (Follower)  │     │ (Follower)  │
└─────────────┘     └─────────────┘     └─────────────┘
```

**Characteristics**:
- Any node can be leader
- Reads distributed across all nodes
- Writes go to leader (redirected automatically)
- Failover time: <1 minute

**Configuration**:
```sql
ALTER SYSTEM SET ha_mode = 'active_active';
ALTER SYSTEM SET min_quorum_size = 2;  -- (N/2 + 1)
ALTER SYSTEM SET read_from_followers = true;
```

**Use Cases**:
- Balanced workloads (70% read, 30% write)
- Need for read scaling
- Automatic failover requirements

### 3. RAC (Real Application Clusters)

See [RAC.md](./RAC.md) for detailed documentation.

**Quick Overview**:
- All nodes active for reads AND writes
- Cache Fusion for memory-to-memory block transfers
- Global Resource Directory for coordination
- Failover time: <30 seconds

### 4. Multi-Region

**Architecture**:
```
  ┌────────────────────────┐
  │      US West           │
  │  ┌──────┐  ┌──────┐   │
  │  │Node1 │  │Node2 │   │──────┐
  │  └──────┘  └──────┘   │      │
  └────────────────────────┘      │
                                  │ Async
  ┌────────────────────────┐      │ Replication
  │      US East           │      │
  │  ┌──────┐  ┌──────┐   │◄─────┘
  │  │Node3 │  │Node4 │   │
  │  └──────┘  └──────┘   │
  └────────────────────────┘
```

**Characteristics**:
- Cross-region replication (async recommended)
- Region-aware client routing
- Independent region failover
- Failover time: <5 minutes (regional)

**Configuration**:
```sql
ALTER SYSTEM SET ha_mode = 'multi_region';

-- Define regions
ALTER SYSTEM ADD REGION 'us-west'
  WITH NODES ('node1', 'node2');

ALTER SYSTEM ADD REGION 'us-east'
  WITH NODES ('node3', 'node4');

-- Set replication
ALTER SYSTEM SET inter_region_replication = 'async';
ALTER SYSTEM SET region_failover_priority = 'us-west,us-east';
```

**Use Cases**:
- Global applications
- Disaster recovery (cross-region)
- Compliance (data residency)

---

## Failover Mechanisms

### Automatic Failover

**Trigger Conditions**:
- Leader heartbeat timeout (5 seconds)
- Leader health check failure (3 consecutive)
- Manual intervention
- Network partition with quorum loss

**Failover Process**:

```
1. Detect Failure
   ├─ Heartbeat timeout
   ├─ Health check failure
   └─ Split-brain detection
         │
         ▼
2. Initiate Election
   ├─ Candidates start campaign
   ├─ Request votes from peers
   └─ Collect quorum
         │
         ▼
3. Elect New Leader
   ├─ Candidate with most votes wins
   ├─ Log consistency check
   └─ Catch up missing entries
         │
         ▼
4. Promote to Leader
   ├─ Start accepting writes
   ├─ Update cluster topology
   └─ Notify clients
         │
         ▼
5. Stabilize Cluster
   ├─ Replicate pending transactions
   ├─ Rebuild failed node (optional)
   └─ Resume normal operations
```

**Failover Times**:
| Cluster Type | Detection | Election | Total |
|--------------|-----------|----------|-------|
| 2-Node (Active-Passive) | 5s | N/A | 5-10s |
| 3-Node Cluster | 5s | 2-5s | 7-15s |
| RAC Cluster | 1s | 1-3s | 2-10s |

### Manual Failover

**Planned Maintenance**:
```sql
-- Graceful shutdown of leader
ALTER SYSTEM PREPARE FOR FAILOVER TO 'node2';

-- Initiate manual failover
ALTER SYSTEM FAILOVER TO 'node2';
```

**Emergency Failover**:
```sql
-- Force failover (skip safety checks)
ALTER SYSTEM FORCE FAILOVER TO 'node2';

-- Reinitialize failed node
ALTER SYSTEM REINITIALIZE NODE 'node1';
```

### Split-Brain Prevention

**Quorum Requirement**:
- Cluster requires (N/2 + 1) nodes for operations
- Example: 3-node cluster needs 2 nodes (majority)
- Minority partition becomes read-only

**Fencing**:
```sql
-- Configure fencing
ALTER SYSTEM SET fencing_mode = 'automatic';
ALTER SYSTEM SET fencing_method = 'power_management'; -- or 'network_isolation'

-- Manual fencing
ALTER SYSTEM FENCE NODE 'node3';
```

**Network Partition Handling**:
```
Before Partition:
┌────┐  ┌────┐  ┌────┐
│ N1 │──│ N2 │──│ N3 │
└────┘  └────┘  └────┘
   Leader

After Partition:
┌────┐  ┌────┐     ││     ┌────┐
│ N1 │──│ N2 │     ││     │ N3 │
└────┘  └────┘     ││     └────┘
Majority (R/W)   Partition   Minority (R/O)
```

---

## Load Balancing

### Connection Load Balancing

**Round Robin**:
```sql
ALTER SYSTEM SET load_balance_mode = 'round_robin';
```

**Least Connections**:
```sql
ALTER SYSTEM SET load_balance_mode = 'least_connections';
```

**Weighted**:
```sql
ALTER SYSTEM SET load_balance_mode = 'weighted';

-- Set weights per node
ALTER NODE 'node1' SET WEIGHT 3;  -- 3x capacity
ALTER NODE 'node2' SET WEIGHT 2;
ALTER NODE 'node3' SET WEIGHT 1;
```

**Health-Based**:
```sql
ALTER SYSTEM SET load_balance_mode = 'health_based';

-- Only route to nodes with health score > 80%
ALTER SYSTEM SET lb_min_health_score = 80;
```

### Query Load Balancing

**Read/Write Splitting**:
```sql
-- Route reads to followers, writes to leader
ALTER SYSTEM SET query_routing = 'read_write_split';

-- Optional: read from leader for strong consistency
SET SESSION read_from_leader = true;
```

**Workload-Based**:
```sql
-- OLTP queries to node1, OLAP to node2
CREATE WORKLOAD POLICY oltp_policy
  FOR WORKLOAD_TYPE = 'OLTP'
  ROUTE TO NODES ('node1', 'node3');

CREATE WORKLOAD POLICY olap_policy
  FOR WORKLOAD_TYPE = 'OLAP'
  ROUTE TO NODES ('node2');
```

### Client-Side Load Balancing

**Connection String**:
```
postgresql://rustydb:password@node1:5432,node2:5432,node3:5432/dbname?target_session_attrs=read-write&load_balance_hosts=true
```

**Configuration**:
- `target_session_attrs=read-write`: Connect to leader
- `target_session_attrs=any`: Connect to any node
- `load_balance_hosts=true`: Randomize connection order

---

## Distributed Query Execution

### Query Coordinator

**Architecture**:
```
Client Query → Coordinator Node → Parse & Plan
                                       │
                       ┌───────────────┼───────────────┐
                       ▼               ▼               ▼
                   Node 1          Node 2          Node 3
                   Scan T1         Scan T2         Scan T3
                       │               │               │
                       └───────────────┼───────────────┘
                                       ▼
                              Merge Results
                                       │
                                       ▼
                                  Return to Client
```

**Supported Operations**:
- Distributed joins
- Parallel aggregations
- Cross-shard queries
- Distributed transactions (2PC)

**Configuration**:
```sql
-- Enable distributed query execution
ALTER SYSTEM SET distributed_query = true;

-- Set parallelism per query
SET SESSION parallel_workers = 4;

-- Enable cost-based decision
ALTER SYSTEM SET distributed_query_threshold = 1000; -- rows
```

### Parallel Execution

**Intra-Query Parallelism**:
```sql
-- Parallel scan
EXPLAIN (ANALYZE) SELECT * FROM large_table;
```

**Inter-Node Parallelism**:
```sql
-- Query executed across all nodes
SELECT /*+ PARALLEL(4) */ count(*)
FROM distributed_table;
```

---

## Data Migration and Rebalancing

### Online Data Migration

**Scenario**: Add new node to cluster

```sql
-- Add new node
ALTER SYSTEM ADD NODE 'node4'
  AT 'node4.example.com:5432'
  WITH ROLE = 'follower';

-- Initiate rebalancing
ALTER SYSTEM REBALANCE DATA
  FROM NODES ('node1', 'node2', 'node3')
  TO NODE 'node4'
  WITH STRATEGY = 'gradual';  -- or 'aggressive'
```

**Rebalancing Strategies**:

**Gradual** (default):
- Low impact on production traffic
- 10% of resources for migration
- Longer completion time

**Aggressive**:
- Higher impact, faster completion
- 50% of resources for migration
- Recommended for off-peak hours

**Progressive**:
- Start gradual, increase if cluster idle
- Adaptive resource allocation

### Data Movement

**Chunk-Based Migration**:
```sql
-- Monitor rebalancing progress
SELECT * FROM system.rebalancing_status;

-- Output:
-- chunk_id | source_node | target_node | progress | status
-- 1001     | node1       | node4       | 45%      | moving
-- 1002     | node2       | node4       | 100%     | complete
```

**Online Operations**:
- Queries continue during migration
- Dual-write to old and new locations
- Atomic switchover after sync complete

---

## Monitoring and Health Checks

### Cluster Health

**Health Score Calculation**:
```
Health Score = (
  CPU Health (0-100) * 0.3 +
  Memory Health (0-100) * 0.2 +
  Disk Health (0-100) * 0.2 +
  Network Health (0-100) * 0.15 +
  Replication Health (0-100) * 0.15
)
```

**Query Health Status**:
```sql
SELECT * FROM system.cluster_health;

-- Output:
-- node_id | role      | health_score | status  | uptime
-- node1   | leader    | 95           | healthy | 7d
-- node2   | follower  | 88           | healthy | 7d
-- node3   | follower  | 92           | healthy | 6d
```

### Key Metrics

**Cluster Metrics**:
- Total nodes, healthy nodes, failed nodes
- Quorum status (has_quorum: true/false)
- Current leader
- Election count (frequent elections indicate instability)

**Node Metrics**:
- CPU usage, memory usage, disk I/O
- Network throughput
- Query latency (p50, p95, p99)
- Connection count

**Replication Metrics**:
- Replication lag (ms)
- Bytes replicated per second
- Log position delta

### Alerts

**Critical Alerts**:
```sql
-- Quorum loss
ALTER SYSTEM CREATE ALERT quorum_loss
  WHEN cluster.has_quorum = false
  NOTIFY 'email:oncall@example.com';

-- Node failure
ALTER SYSTEM CREATE ALERT node_failure
  WHEN node.status = 'failed'
  NOTIFY 'slack:#database-alerts';
```

**Warning Alerts**:
```sql
-- High replication lag
ALTER SYSTEM CREATE ALERT high_replication_lag
  WHEN replication.lag_ms > 5000
  NOTIFY 'pagerduty:db_team';

-- Low health score
ALTER SYSTEM CREATE ALERT low_health
  WHEN node.health_score < 70
  NOTIFY 'email:dba@example.com';
```

---

## Configuration Guide

### Minimal 3-Node Cluster

**Node 1** (`rustydb.conf`):
```toml
[cluster]
enabled = true
node_id = "node1"
node_address = "node1.example.com:5432"
raft_port = 5433

[cluster.raft]
election_timeout_ms = 5000
heartbeat_interval_ms = 1000

[cluster.nodes]
node1 = "node1.example.com:5433"
node2 = "node2.example.com:5433"
node3 = "node3.example.com:5433"

[ha]
mode = "active_active"
auto_failover = true
min_quorum_size = 2
```

**Node 2, Node 3**: Same configuration with respective `node_id`

### Production 5-Node Cluster

```toml
[cluster]
enabled = true
cluster_name = "production_cluster"
replication_factor = 3
geo_replication = true

[cluster.regions]
us_west = ["node1", "node2"]
us_east = ["node3", "node4"]
eu_west = ["node5"]

[ha]
mode = "multi_region"
auto_failover = true
failover_timeout_ms = 30000
fencing_mode = "automatic"

[load_balancing]
mode = "health_based"
min_health_score = 80
read_from_followers = true
```

---

## Operational Procedures

### Adding a Node

```bash
# 1. Prepare new server
$ rustydb-admin node init \
  --node-id node4 \
  --cluster production_cluster

# 2. Join cluster
$ rustydb-admin cluster join \
  --node-id node4 \
  --cluster-address node1.example.com:5433

# 3. Verify
$ rustydb-admin cluster status
```

### Removing a Node

```bash
# 1. Drain connections
$ rustydb-admin node drain node4

# 2. Remove from cluster
$ rustydb-admin cluster remove node4

# 3. Shutdown
$ rustydb-admin node stop node4
```

### Rolling Upgrade

```bash
# Upgrade follower nodes first
for node in node2 node3; do
  rustydb-admin node drain $node
  rustydb-admin node upgrade $node --version 0.6.1
  rustydb-admin node start $node
  sleep 60  # Wait for stability
done

# Trigger failover to upgraded node
rustydb-admin cluster failover --to node2

# Upgrade old leader
rustydb-admin node drain node1
rustydb-admin node upgrade node1 --version 0.6.1
rustydb-admin node start node1
```

---

## Troubleshooting

### Frequent Leader Elections

**Symptoms**: Leader changes multiple times per hour

**Causes**:
- Network instability
- Overloaded leader
- Timeout too aggressive

**Solutions**:
```sql
-- Increase election timeout
ALTER SYSTEM SET election_timeout_ms = 10000;

-- Check network latency
SELECT * FROM system.network_latency;

-- Review leader load
SELECT * FROM system.node_stats WHERE node_id = 'leader';
```

### Quorum Loss

**Symptoms**: Cluster becomes read-only

**Cause**: Majority of nodes offline

**Recovery**:
```bash
# Check cluster status
$ rustydb-admin cluster status

# Force quorum with minority (DANGER: data loss risk)
$ rustydb-admin cluster force-quorum \
  --nodes node1,node2 \
  --confirm-data-loss

# Reinitialize failed nodes
$ rustydb-admin node reinit node3
```

### High Replication Lag

**Symptoms**: Standby falling behind primary

**Causes**:
- Network bandwidth limitation
- Standby slower than primary
- High write volume

**Solutions**:
```sql
-- Check replication stats
SELECT * FROM system.replication_stats;

-- Increase network buffer
ALTER SYSTEM SET wal_sender_buffer_size = '128MB';

-- Enable compression
ALTER SYSTEM SET wal_compression = true;
```

---

## Best Practices

1. **Always use odd number of nodes** for Raft clusters (3, 5, 7)
2. **Monitor replication lag** and set alerts for lag > 5 seconds
3. **Test failover regularly** (monthly recommended)
4. **Use synchronous replication** for zero data loss requirements
5. **Spread nodes across failure domains** (different racks/zones)
6. **Set appropriate timeouts** based on network latency
7. **Monitor cluster health continuously**
8. **Document runbooks** for common failure scenarios
9. **Perform rolling upgrades** during low-traffic periods
10. **Backup cluster configuration** before major changes

---

**Next Steps**:
- [RAC Configuration](./RAC.md) - For RAC-specific clustering
- [Replication Setup](./REPLICATION.md) - For replication configuration
- [Monitoring Guide](../operations/MONITORING.md) - For comprehensive monitoring

**Document Version**: 1.0
**Last Updated**: December 2025
