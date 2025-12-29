# RustyDB v0.6.5 Resource Governance & Workload Management

**Version**: 0.6.5
**Last Updated**: December 2025
**Target Audience**: Database Administrators, Performance Engineers
**Status**: ✅ **Validated for Enterprise Deployment**

---

## Overview

RustyDB Resource Governance provides fine-grained control over database resource allocation, enabling workload consolidation and quality-of-service (QoS) guarantees for multi-tenant and mixed workload environments.

### Key Features ✅

- Resource groups with CPU, memory, and I/O limits
- Query prioritization and throttling
- Automatic workload classification
- Session-level resource control
- Real-time resource monitoring
- Adaptive resource allocation

---

## Resource Groups

### Configuration

**Resource Group Types**:

| Group | CPU Share | Memory Limit | Max Parallelism | Session Timeout | Use Case |
|-------|-----------|--------------|-----------------|-----------------|----------|
| **CRITICAL** | 40% | Unlimited | 32 | None | Mission-critical OLTP |
| **HIGH** | 30% | 50% | 16 | 1 hour | Important batch jobs |
| **MEDIUM** | 20% | 30% | 8 | 30 min | Standard queries |
| **LOW** | 10% | 20% | 4 | 15 min | Reports, analytics |

### API Configuration

```sql
-- Create resource group
CREATE RESOURCE GROUP critical_oltp WITH (
    cpu_share = 40,
    memory_limit_percent = 0,  -- Unlimited
    max_parallel_degree = 32,
    max_execution_time_sec = 0,  -- No limit
    max_idle_time_sec = 0
);

-- Assign user to resource group
ALTER USER admin SET RESOURCE GROUP critical_oltp;

-- Assign session to resource group
SET SESSION RESOURCE GROUP high_priority;
```

---

## Resource Allocation

### CPU Allocation

**Fair-Share Scheduling**:
```
CPU allocation = (group_share / total_shares) * available_CPU
```

**Example**:
- CRITICAL: 40 shares → 40% CPU
- HIGH: 30 shares → 30% CPU
- MEDIUM: 20 shares → 20% CPU
- LOW: 10 shares → 10% CPU

**Under Load**:
- Guaranteed minimum based on shares
- Unused capacity redistributed proportionally
- Throttling when exceeding quota

### Memory Allocation

**Hierarchical Limits**:
```
Global Memory
  ├─ CRITICAL: 40%
  ├─ HIGH: 30%
  ├─ MEDIUM: 20%
  └─ LOW: 10%
```

**Memory Types**:
- **Sort area**: Temporary space for sorting operations
- **Hash area**: Memory for hash joins
- **Work area**: General query execution memory
- **Buffer pool**: Shared buffer cache allocation

### I/O Allocation

**IOPS Limits**:
- Per-group IOPS quotas
- Token bucket algorithm
- Burst capacity (2x quota for 5 seconds)
- Priority-based scheduling

---

## Query Prioritization

### Priority Levels

```rust
pub enum QueryPriority {
    Critical = 4,  // Guaranteed resources
    High = 3,      // Preferential treatment
    Normal = 2,    // Standard processing
    Low = 1,       // Best effort
}
```

### Automatic Classification

**Workload Detection**:
```sql
-- OLTP: Short queries, high frequency
SELECT * FROM orders WHERE order_id = ?;
→ Priority: HIGH

-- OLAP: Long-running analytics
SELECT region, SUM(revenue) FROM sales GROUP BY region;
→ Priority: LOW

-- Admin: DDL operations
CREATE INDEX idx_customer_email ON customers(email);
→ Priority: NORMAL
```

**Classification Rules**:
1. Query complexity (joins, aggregations)
2. Estimated row count
3. Execution time history
4. User/session priority
5. Time of day (business hours vs. off-hours)

---

## Workload Management

### Session Control

**Session-Level Limits**:
```sql
-- Set session timeout
SET SESSION max_idle_time = 300;  -- 5 minutes

-- Set query timeout
SET SESSION max_execution_time = 60;  -- 1 minute

-- Set memory limit
SET SESSION max_memory = '2GB';

-- Set parallelism
SET SESSION parallel_degree = 8;
```

### Query Throttling

**Rate Limiting**:
```sql
-- Limit queries per second per user
CREATE RATE LIMIT user_queries WITH (
    max_queries_per_second = 100,
    max_concurrent_queries = 10
);

ALTER USER analytics_user SET RATE LIMIT user_queries;
```

**Queue Management**:
- Active query slots (max concurrent)
- Wait queue with timeout
- Priority-based dequeuing
- Automatic retry on failure

---

## Monitoring & Observability

### Resource Usage Metrics

```sql
-- Current resource usage by group
SELECT
    resource_group,
    active_sessions,
    cpu_usage_percent,
    memory_usage_mb,
    iops_current,
    queries_running,
    queries_queued
FROM v$resource_group_stats
ORDER BY cpu_usage_percent DESC;
```

**Output Example**:
```
resource_group | sessions | cpu% | memory  | iops | running | queued
---------------|----------|------|---------|------|---------|-------
CRITICAL       | 45       | 38.2 | 8192 MB | 4500 | 12      | 0
HIGH           | 28       | 29.1 | 4096 MB | 2800 | 8       | 2
MEDIUM         | 102      | 18.7 | 2048 MB | 1200 | 25      | 15
LOW            | 55       | 14.0 | 1024 MB | 500  | 10      | 45
```

### Query Performance Tracking

```sql
-- Top resource-consuming queries
SELECT
    query_id,
    query_text,
    resource_group,
    cpu_time_sec,
    memory_peak_mb,
    io_operations,
    execution_time_sec,
    priority
FROM v$top_queries
WHERE execution_time_sec > 60
ORDER BY cpu_time_sec DESC
LIMIT 10;
```

---

## Performance Tuning

### Optimization Guidelines

**CPU-Bound Workloads**:
- Increase CPU shares for critical groups
- Enable parallel execution
- Review query plans for inefficiencies
- Consider query result caching

**Memory-Bound Workloads**:
- Increase memory limits
- Optimize sort/hash area sizes
- Enable compression
- Review index usage

**I/O-Bound Workloads**:
- Increase IOPS quota
- Enable data compression
- Optimize table partitioning
- Use columnar storage for analytics

### Adaptive Tuning

**Auto-Scaling** (if enabled):
```
IF cpu_usage > 80% FOR 5 minutes THEN
    increase_cpu_allocation(10%)
END IF

IF memory_pressure THEN
    trigger_eviction()
    alert_administrator()
END IF
```

---

## Integration with Multi-Tenancy

### Tenant-Level Resource Groups

```rust
// Automatic resource group assignment per tenant tier

ServiceTier::Bronze → ResourceGroup::LOW
ServiceTier::Silver → ResourceGroup::MEDIUM
ServiceTier::Gold → ResourceGroup::HIGH
ServiceTier::Platinum → ResourceGroup::CRITICAL
```

**Example**:
```sql
-- Tenant acme_corp (Bronze tier) queries automatically assigned to LOW group
-- CPU: 10% share
-- Memory: 20% limit
-- Parallelism: 4
-- Timeout: 15 minutes
```

---

## Best Practices

### Production Configuration

**Recommended Setup**:
1. Define 4-5 resource groups aligned with SLAs
2. Assign critical applications to CRITICAL group
3. Use MEDIUM as default for unknown workloads
4. Monitor resource group utilization daily
5. Adjust allocations based on actual usage patterns

**Capacity Planning**:
- Reserve 20% capacity for bursts
- Set aggressive timeouts for LOW priority
- Enable query result caching for repeated queries
- Use admission control to prevent overload

### Troubleshooting

**High Queue Depth**:
- Increase active query slots
- Review slow queries (> 1 minute)
- Consider upgrading hardware
- Redistribute workload across time

**Resource Starvation**:
- Check CPU/memory/IOPS allocation fairness
- Verify no runaway queries
- Review priority assignments
- Enable automatic workload classification

---

## Conclusion

RustyDB v0.6.5 Resource Governance provides **enterprise-grade workload management** with:
- ✅ Fine-grained resource control
- ✅ Automatic workload classification
- ✅ Multi-tenant isolation
- ✅ Real-time monitoring
- ✅ Adaptive tuning

**Status**: Production-ready for mixed workload environments

---

**Document Version**: 0.6.5
**Last Updated**: December 2025
**Validation**: ✅ Production Ready

---
