# RustyDB v0.6 Multi-Tenant Architecture

**Version**: 0.6.0
**Last Updated**: December 2025
**Target Audience**: SaaS Architects, Cloud Platform Engineers, DBAs

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture Models](#architecture-models)
3. [PDB/CDB Architecture (Oracle-Style)](#pdbcdb-architecture-oracle-style)
4. [Cloud Multi-Tenancy](#cloud-multi-tenancy)
5. [Resource Isolation](#resource-isolation)
6. [Service Tiers](#service-tiers)
7. [Tenant Lifecycle Management](#tenant-lifecycle-management)
8. [Workload Consolidation](#workload-consolidation)
9. [SLA Management](#sla-management)
10. [Configuration Examples](#configuration-examples)
11. [Best Practices](#best-practices)

---

## Overview

RustyDB v0.6 provides two comprehensive multi-tenancy implementations designed for different use cases:

1. **Oracle-Style PDB/CDB**: Enterprise-familiar pluggable database architecture
2. **Cloud Multi-Tenancy**: Modern service-tier-based multi-tenancy with resource isolation

Both approaches provide **complete tenant isolation**, **resource governance**, and **operational efficiency** for hosting multiple tenants on shared infrastructure.

### Key Benefits

**Cost Efficiency**:
- 10-100x consolidation ratios
- Shared infrastructure reduces cost per tenant
- Economies of scale for operations

**Operational Simplicity**:
- Centralized management
- Automated provisioning (<5 minutes)
- Unified monitoring and backups

**Security & Isolation**:
- Complete data isolation per tenant
- Resource limits prevent noisy neighbors
- Separate security policies per tenant

**Scalability**:
- Support for 1000+ tenants per instance
- Elastic resource allocation
- Automated tier upgrades/downgrades

---

## Architecture Models

### Comparison

| Feature | PDB/CDB | Cloud Multi-Tenancy |
|---------|---------|---------------------|
| **Origin** | Oracle Multitenant | Cloud-native design |
| **Tenant Unit** | Pluggable Database (PDB) | Tenant with service tier |
| **Isolation** | Database-level | Resource-level (CPU, memory, I/O) |
| **Management** | SQL-based (ALTER PLUGGABLE DATABASE) | API-based (REST/GraphQL) |
| **Service Tiers** | Not built-in | Bronze, Silver, Gold, Platinum |
| **Shared Services** | Undo, temp, common users | Resource pools |
| **Best For** | Oracle compatibility, lift-and-shift | SaaS platforms, cloud services |

### When to Use Each

**PDB/CDB**:
- Oracle database migration
- Enterprise familiar with Oracle Multitenant
- Need for hot cloning and relocation
- Regulatory requirement for database-level isolation

**Cloud Multi-Tenancy**:
- SaaS platform development
- Cloud service providers
- Need for service tiers and SLA management
- Dynamic resource scaling

---

## PDB/CDB Architecture (Oracle-Style)

### Concept

A **Container Database (CDB)** hosts multiple **Pluggable Databases (PDBs)**, each appearing as a fully independent database to applications while sharing the underlying infrastructure.

```
┌───────────────────────────────────────────────────────┐
│          Container Database (CDB)                      │
├───────────────────────────────────────────────────────┤
│  CDB$ROOT                                             │
│  - Common users (C##*)                                │
│  - Common roles                                       │
│  - Shared services (undo, temp)                       │
└───────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   PDB_001    │ │   PDB_002    │ │   PDB_003    │
│  (Tenant A)  │ │  (Tenant B)  │ │  (Tenant C)  │
├──────────────┤ ├──────────────┤ ├──────────────┤
│ • Local users│ │ • Local users│ │ • Local users│
│ • Schemas    │ │ • Schemas    │ │ • Schemas    │
│ • Datafiles  │ │ • Datafiles  │ │ • Datafiles  │
└──────────────┘ └──────────────┘ └──────────────┘
```

### PDB Operations

**Create PDB**:
```sql
-- Create empty PDB
CREATE PLUGGABLE DATABASE pdb_tenant1
  ADMIN USER pdb_admin IDENTIFIED BY 'secure_password'
  FILE_NAME_CONVERT = ('/pdbseed/', '/pdb_tenant1/');

-- Create from seed (template)
CREATE PLUGGABLE DATABASE pdb_tenant2
  FROM pdb_seed
  FILE_NAME_CONVERT = ('/pdbseed/', '/pdb_tenant2/');

-- Clone existing PDB (hot clone)
CREATE PLUGGABLE DATABASE pdb_tenant3
  FROM pdb_tenant1
  FILE_NAME_CONVERT = ('/pdb_tenant1/', '/pdb_tenant3/');
```

**Open/Close PDB**:
```sql
-- Open PDB
ALTER PLUGGABLE DATABASE pdb_tenant1 OPEN;

-- Open in read-only mode
ALTER PLUGGABLE DATABASE pdb_tenant1 OPEN READ ONLY;

-- Close PDB
ALTER PLUGGABLE DATABASE pdb_tenant1 CLOSE IMMEDIATE;
```

**Modify PDB**:
```sql
-- Set resource limits
ALTER PLUGGABLE DATABASE pdb_tenant1
  SET RESOURCE LIMITS
    CPU_COUNT = 4,
    MEMORY_SIZE = 8GB,
    MAX_CONNECTIONS = 100;

-- Rename PDB
ALTER PLUGGABLE DATABASE pdb_tenant1 RENAME TO pdb_customer_a;
```

**Unplug/Plug**:
```sql
-- Unplug PDB (export metadata)
ALTER PLUGGABLE DATABASE pdb_tenant1 CLOSE;
ALTER PLUGGABLE DATABASE pdb_tenant1
  UNPLUG INTO '/backup/pdb_tenant1.xml';

-- Drop unplugged PDB
DROP PLUGGABLE DATABASE pdb_tenant1 KEEP DATAFILES;

-- Plug PDB into different CDB
CREATE PLUGGABLE DATABASE pdb_tenant1
  USING '/backup/pdb_tenant1.xml'
  MOVE;
```

**Drop PDB**:
```sql
-- Drop with data files
ALTER PLUGGABLE DATABASE pdb_tenant1 CLOSE;
DROP PLUGGABLE DATABASE pdb_tenant1 INCLUDING DATAFILES;

-- Drop but keep data files (for backup)
DROP PLUGGABLE DATABASE pdb_tenant1 KEEP DATAFILES;
```

### Shared Services

**Shared Undo Tablespace**:
- All PDBs share undo tablespace in CDB$ROOT
- Reduces storage overhead
- Centralized transaction management

**Shared Temporary Tablespace**:
- All PDBs share temp tablespace
- Dynamic space allocation

**Common Users**:
```sql
-- Create common user (accessible in all PDBs)
CREATE USER C##admin IDENTIFIED BY 'password' CONTAINER=ALL;
GRANT DBA TO C##admin CONTAINER=ALL;

-- Local user (specific to PDB)
ALTER SESSION SET CONTAINER = pdb_tenant1;
CREATE USER tenant1_user IDENTIFIED BY 'password';
```

### Lockdown Profiles

Restrict operations available within PDBs for security:

```sql
-- Create lockdown profile
CREATE LOCKDOWN PROFILE tenant_restrictions;

-- Disable specific features
ALTER LOCKDOWN PROFILE tenant_restrictions
  DISABLE STATEMENT = ('ALTER SYSTEM');

ALTER LOCKDOWN PROFILE tenant_restrictions
  DISABLE FEATURE = ('FILE_IO', 'NETWORK_ACCESS');

-- Apply to PDB
ALTER PLUGGABLE DATABASE pdb_tenant1
  SET LOCKDOWN_PROFILE = tenant_restrictions;
```

---

## Cloud Multi-Tenancy

### Service Tiers

RustyDB provides four pre-defined service tiers with guaranteed resources and SLAs:

| Tier | CPU | Memory | Storage | Max Connections | Price/Month | SLA Uptime |
|------|-----|--------|---------|----------------|-------------|-----------|
| **Bronze** | 1 vCPU | 2 GB | 50 GB | 50 | $100 | 99.0% |
| **Silver** | 2 vCPU | 4 GB | 100 GB | 100 | $250 | 99.5% |
| **Gold** | 4 vCPU | 8 GB | 250 GB | 200 | $500 | 99.9% |
| **Platinum** | 8 vCPU | 16 GB | 500 GB | 500 | $1,000 | 99.99% |

### Tenant Creation

```rust
// Create tenant via API
POST /api/v1/tenants
{
  "tenant_name": "acme_corp",
  "admin_user": "admin@acme.com",
  "service_tier": "gold",
  "priority": "high",
  "metadata": {
    "company": "Acme Corp",
    "industry": "Manufacturing"
  }
}

// Response
{
  "tenant_id": "tenant_12345",
  "status": "active",
  "service_tier": "gold",
  "resources": {
    "cpu": 4,
    "memory_gb": 8,
    "storage_gb": 250,
    "max_connections": 200
  },
  "created_at": "2025-12-28T10:00:00Z"
}
```

### Tenant Management

```sql
-- List all tenants
SELECT * FROM system.tenants;

-- Suspend tenant
ALTER TENANT acme_corp SUSPEND;

-- Resume tenant
ALTER TENANT acme_corp RESUME;

-- Upgrade tier
ALTER TENANT acme_corp SET SERVICE_TIER = 'platinum';

-- Modify resources (manual override)
ALTER TENANT acme_corp
  SET CPU_COUNT = 6,
      MEMORY_SIZE = '12GB';
```

---

## Resource Isolation

### Memory Isolation

**Per-Tenant Quotas**:
```sql
-- Set memory limit
ALTER TENANT acme_corp SET MEMORY_QUOTA = '8GB';

-- Query memory usage
SELECT tenant_id,
       memory_allocated_mb,
       memory_used_mb,
       memory_peak_mb,
       oom_events
FROM system.tenant_memory_usage;
```

**OOM Protection**:
- Tenant exceeding quota is throttled (queries slower)
- OOM events logged and alerted
- Global limit prevents system-wide OOM

### CPU Isolation

**Fair Share Scheduling**:
```sql
-- Set CPU limits
ALTER TENANT acme_corp
  SET CPU_MIN_PERCENT = 25,  -- Guaranteed minimum
      CPU_MAX_PERCENT = 50;  -- Maximum burst

-- CPU usage tracking
SELECT tenant_id,
       cpu_usage_percent,
       cpu_throttled_ms,
       cpu_wait_ms
FROM system.tenant_cpu_usage;
```

**CPU Scheduling Algorithm**:
- Fair share based on service tier
- Min/max enforcement
- Priority levels (Critical > High > Medium > Low)

### I/O Bandwidth Isolation

**Token Bucket Rate Limiting**:
```sql
-- Set I/O limits (IOPS)
ALTER TENANT acme_corp
  SET IOPS_LIMIT = 10000,
      IOPS_BURST = 20000;

-- I/O statistics
SELECT tenant_id,
       read_iops,
       write_iops,
       throttled_iops,
       avg_latency_ms
FROM system.tenant_io_stats;
```

### Network Isolation

**Dedicated Port Allocation**:
```sql
-- Assign dedicated port (optional)
ALTER TENANT acme_corp SET PORT = 5433;

-- Network bandwidth limits
ALTER TENANT acme_corp
  SET NETWORK_BANDWIDTH_MBPS = 1000,
      NETWORK_BURST_MBPS = 2000;
```

### Buffer Pool Isolation

**Per-Tenant Buffer Cache**:
```sql
-- Set buffer pool quota
ALTER TENANT acme_corp SET BUFFER_POOL_SIZE = '2GB';

-- Buffer pool statistics
SELECT tenant_id,
       buffer_pool_size_mb,
       cache_hit_ratio,
       dirty_pages,
       evictions
FROM system.tenant_buffer_stats;
```

### Lock Contention Isolation

**Timeout Enforcement**:
```sql
-- Set lock timeout per tenant
ALTER TENANT acme_corp SET LOCK_TIMEOUT = 30000;  -- milliseconds

-- Lock contention tracking
SELECT tenant_id,
       locks_held,
       locks_waiting,
       avg_wait_time_ms,
       deadlocks
FROM system.tenant_lock_stats;
```

---

## Service Tiers

### Tier Characteristics

**Bronze** (Entry-level):
- Small applications, development/testing
- Basic resource guarantees
- Best-effort performance
- 99.0% uptime SLA

**Silver** (Standard):
- Production workloads
- Moderate resource guarantees
- Good performance
- 99.5% uptime SLA

**Gold** (Premium):
- Mission-critical applications
- Strong resource guarantees
- High performance
- 99.9% uptime SLA (43.8 min downtime/month)

**Platinum** (Enterprise):
- Enterprise-grade applications
- Maximum resources
- Highest priority
- 99.99% uptime SLA (4.38 min downtime/month)

### Tier Upgrades/Downgrades

```sql
-- Upgrade tier
ALTER TENANT acme_corp UPGRADE TO TIER 'platinum';

-- Downgrade tier (requires confirmation)
ALTER TENANT acme_corp DOWNGRADE TO TIER 'silver'
  CONFIRM_RESOURCE_REDUCTION;

-- View tier change history
SELECT * FROM system.tenant_tier_history
WHERE tenant_id = 'acme_corp'
ORDER BY changed_at DESC;
```

---

## Tenant Lifecycle Management

### Provisioning Workflow

```
1. Request Tenant Creation
   ├─ Validate input (tier, resources, name)
   ├─ Check capacity (can we accommodate?)
   └─ Allocate tenant ID
         │
         ▼
2. Provision Resources
   ├─ Allocate CPU, memory, storage
   ├─ Create network namespace
   └─ Initialize buffer pool
         │
         ▼
3. Create Database Objects
   ├─ Create schemas
   ├─ Initialize metadata
   └─ Set up security policies
         │
         ▼
4. Configure Isolation
   ├─ Apply resource limits
   ├─ Set up network isolation
   └─ Configure monitoring
         │
         ▼
5. Activate Tenant
   ├─ Mark as ACTIVE
   ├─ Enable connections
   └─ Notify admin
```

**Typical Provisioning Time**: <5 minutes

### Deprovisioning

```sql
-- Graceful shutdown
ALTER TENANT acme_corp SUSPEND;

-- Backup before deletion
SELECT backup_tenant('acme_corp', '/backups/acme_corp_final.tar.gz');

-- Delete tenant
DROP TENANT acme_corp
  WITH BACKUP_RETENTION = 90;  -- days
```

---

## Workload Consolidation

### Intelligent Placement

**Workload Types**:
- OLTP: High concurrency, short transactions
- OLAP: Low concurrency, long analytical queries
- Mixed: Combination of OLTP and OLAP
- Batch: Periodic batch processing

**Placement Strategy**:
```sql
-- Tag tenant workload type
ALTER TENANT acme_corp SET WORKLOAD_TYPE = 'OLAP';

-- Affinity rules
CREATE CONSOLIDATION RULE oltp_isolation
  WHERE workload_type = 'OLTP'
  AFFINITY HOST_AFFINITY;  -- Keep on same host

CREATE CONSOLIDATION RULE olap_spread
  WHERE workload_type = 'OLAP'
  AFFINITY ANTI_HOST_AFFINITY;  -- Spread across hosts
```

### Consolidation Metrics

```sql
-- Consolidation ratio
SELECT host_id,
       count(*) as tenant_count,
       sum(cpu_used) / max(cpu_total) as cpu_utilization,
       sum(memory_used_mb) / max(memory_total_mb) as memory_utilization
FROM system.tenant_placement
GROUP BY host_id;
```

---

## SLA Management

### SLA Monitoring

```sql
-- Tenant SLA status
SELECT tenant_id,
       service_tier,
       sla_target_percent,
       sla_actual_percent,
       uptime_seconds,
       downtime_seconds,
       violations_count
FROM system.tenant_sla;

-- SLA violations
SELECT tenant_id,
       violation_type,
       started_at,
       ended_at,
       duration_seconds,
       severity
FROM system.sla_violations
WHERE tenant_id = 'acme_corp'
ORDER BY started_at DESC;
```

### Automated Remediation

```sql
-- Create SLA alert
CREATE ALERT sla_violation
  WHEN sla_actual_percent < sla_target_percent
  NOTIFY 'ops@example.com'
  EXECUTE PROCEDURE remediate_sla_violation(tenant_id);

-- Remediation procedure
CREATE PROCEDURE remediate_sla_violation(p_tenant_id TEXT) AS $$
BEGIN
  -- Increase resource allocation temporarily
  PERFORM boost_tenant_resources(p_tenant_id, factor => 1.5);

  -- Alert operations team
  PERFORM send_alert('sla_violation', p_tenant_id);
END;
$$ LANGUAGE plpgsql;
```

---

## Configuration Examples

### Basic Multi-Tenant Setup

```sql
-- Enable multi-tenancy
ALTER SYSTEM SET multi_tenancy_enabled = true;

-- Configure resource pools
ALTER SYSTEM SET tenant_resource_pools = 10;

-- Set global limits
ALTER SYSTEM SET max_tenants = 1000;
ALTER SYSTEM SET tenant_max_cpu_percent = 80;  -- Per tenant
ALTER SYSTEM SET tenant_max_memory_percent = 50;

-- Create tenant
CREATE TENANT startup_co
  WITH SERVICE_TIER = 'silver',
       PRIORITY = 'medium',
       ADMIN_USER = 'admin@startup.co';
```

### High-Density Configuration (1000+ tenants)

```toml
[multitenancy]
enabled = true
max_tenants = 2000
default_tier = "bronze"

[multitenancy.resource_pools]
cpu_pools = 20
memory_pools = 20
io_pools = 20

[multitenancy.limits]
tenant_max_connections = 100
tenant_query_timeout_ms = 300000  # 5 minutes
tenant_idle_timeout_ms = 1800000  # 30 minutes

[multitenancy.isolation]
strict_cpu_isolation = true
memory_oom_protection = true
network_bandwidth_limiting = true
```

---

## Best Practices

1. **Right-Size Service Tiers**: Monitor usage and adjust tiers appropriately
2. **Use Workload Tags**: Tag tenants for intelligent consolidation
3. **Monitor SLA Compliance**: Set alerts for SLA violations
4. **Regular Audits**: Review tenant resource usage monthly
5. **Capacity Planning**: Maintain 20% headroom for growth
6. **Tenant Isolation Testing**: Verify noisy neighbor protection
7. **Backup Strategy**: Per-tenant backups with retention policies
8. **Security**: Use separate credentials per tenant
9. **Metering**: Track usage for billing and chargeback
10. **Automate**: Use APIs for provisioning and management

---

**See Also**:
- [Enterprise Overview](./ENTERPRISE_OVERVIEW.md)
- [Resource Governance](../operations/RESOURCE_GOVERNANCE.md)
- [Multitenancy Test Report](/docs/MULTITENANT_TEST_REPORT.md)

**Document Version**: 1.0
**Last Updated**: December 2025
