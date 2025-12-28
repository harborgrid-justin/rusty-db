# RustyDB Multi-Tenant API Reference

**RustyDB v0.6.0 - Enterprise Server**
**Last Updated**: 2025-12-28
**Status**: PROPOSED (Not yet fully implemented)
**Implementation Priority**: HIGH

---

## Table of Contents

1. [Introduction](#introduction)
2. [Architecture Overview](#architecture-overview)
3. [REST API Endpoints](#rest-api-endpoints)
4. [GraphQL API](#graphql-api)
5. [Resource Management](#resource-management)
6. [Isolation & Security](#isolation--security)
7. [Billing & Metering](#billing--metering)
8. [Migration Guide](#migration-guide)

---

## Introduction

RustyDB provides comprehensive multi-tenant database capabilities inspired by Oracle's Multitenant Architecture, enabling efficient resource sharing and tenant isolation.

### Key Concepts

**Container Database (CDB)**: Root container holding shared services
**Pluggable Database (PDB)**: Isolated tenant database
**Tenant**: Isolated customer environment with dedicated resources
**Service Tier**: Bronze, Silver, Gold, Platinum

### Benefits

| Benefit | Description |
|---------|-------------|
| **Resource Efficiency** | Share infrastructure across tenants |
| **Isolation** | Complete data and resource separation |
| **Scalability** | Thousands of tenants per CDB |
| **Rapid Provisioning** | Clone PDBs in seconds |
| **Cost Savings** | Reduced hardware and management costs |

### Architecture

```
┌──────────────────────────────────────────────┐
│         Container Database (CDB)             │
│  ┌──────────────────────────────────────┐   │
│  │   Shared Services Layer               │   │
│  │   - UNDO Tablespace                   │   │
│  │   - TEMP Tablespace                   │   │
│  │   - Common Users                      │   │
│  │   - Background Processes              │   │
│  └──────────────────────────────────────┘   │
│                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │  PDB$SEED│  │  PDB_1   │  │  PDB_2   │  │
│  │  (Template)│  │ (Tenant) │  │ (Tenant) │  │
│  └──────────┘  └──────────┘  └──────────┘  │
└──────────────────────────────────────────────┘
```

---

## Architecture Overview

### Service Tiers

| Tier | CPU | Memory | Storage | IOPS | Connections | Price |
|------|-----|--------|---------|------|-------------|-------|
| **Bronze** | 1 core | 2 GB | 100 GB | 1,000 | 50 | $$ |
| **Silver** | 2 cores | 4 GB | 500 GB | 3,000 | 100 | $$$ |
| **Gold** | 4 cores | 8 GB | 2 TB | 5,000 | 250 | $$$$ |
| **Platinum** | 8 cores | 16 GB | 10 TB | 10,000 | 500 | $$$$$ |

### Tenant States

| State | Description |
|-------|-------------|
| `ACTIVE` | Tenant is running normally |
| `SUSPENDED` | Tenant temporarily suspended (e.g., payment issue) |
| `MAINTENANCE` | Tenant undergoing maintenance |
| `MIGRATING` | Tenant being migrated |
| `TERMINATED` | Tenant permanently deleted |

---

## REST API Endpoints

### Tenant Management

#### Create Tenant

```http
POST /api/v1/tenants
Authorization: Bearer <token>
Content-Type: application/json

{
  "tenant_name": "acme_corp",
  "admin_user": "admin@acme.com",
  "admin_password": "secure_password",
  "service_tier": "silver",
  "organization": "ACME Corporation",
  "tags": {
    "environment": "production",
    "department": "sales"
  }
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "pdb_id": "PDB_ACME",
    "status": "active",
    "created_at": "2025-12-28T10:00:00Z",
    "resources": {
      "cpu_cores": 2.0,
      "memory_mb": 4096,
      "storage_gb": 500,
      "network_port": 10234
    }
  }
}
```

#### List Tenants

```http
GET /api/v1/tenants?page=1&page_size=20&tier=silver&state=active
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "tenant_id": "tenant_12345",
      "tenant_name": "acme_corp",
      "service_tier": "silver",
      "state": "active",
      "created_at": "2025-12-28T10:00:00Z",
      "usage": {
        "cpu_percent": 45.2,
        "memory_mb": 2048,
        "storage_gb": 150,
        "active_connections": 23
      }
    }
  ],
  "pagination": {
    "total": 42,
    "page": 1,
    "page_size": 20,
    "total_pages": 3
  }
}
```

#### Get Tenant Details

```http
GET /api/v1/tenants/{tenant_id}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "tenant_name": "acme_corp",
    "service_tier": "silver",
    "state": "active",
    "priority": "medium",
    "quota": {
      "cpu_percent": 200,
      "memory_mb": 4096,
      "storage_gb": 500,
      "iops": 3000,
      "max_connections": 100
    },
    "usage": {
      "cpu_percent": 45.2,
      "memory_mb": 2048,
      "storage_gb": 150,
      "iops": 850,
      "active_connections": 23
    },
    "sla": {
      "uptime_percent": 99.7,
      "avg_response_time_ms": 28.5,
      "violations": []
    }
  }
}
```

#### Suspend Tenant

```http
POST /api/v1/tenants/{tenant_id}/suspend
Authorization: Bearer <token>
Content-Type: application/json

{
  "reason": "Payment overdue",
  "notify_admin": true
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "state": "suspended",
    "suspended_at": "2025-12-28T11:00:00Z",
    "reason": "Payment overdue"
  }
}
```

#### Resume Tenant

```http
POST /api/v1/tenants/{tenant_id}/resume
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "state": "active",
    "resumed_at": "2025-12-28T12:00:00Z"
  }
}
```

#### Upgrade Service Tier

```http
PUT /api/v1/tenants/{tenant_id}/tier
Authorization: Bearer <token>
Content-Type: application/json

{
  "new_tier": "gold",
  "effective_date": "immediate"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "old_tier": "silver",
    "new_tier": "gold",
    "upgraded_at": "2025-12-28T13:00:00Z",
    "new_quota": {
      "cpu_cores": 4.0,
      "memory_mb": 8192,
      "storage_gb": 2000
    }
  }
}
```

### PDB Operations

#### Create Pluggable Database

```http
POST /api/v1/pdbs
Authorization: Bearer <token>
Content-Type: application/json

{
  "pdb_name": "PDB_ACME",
  "admin_user": "pdb_admin",
  "admin_password": "secure_pwd",
  "tenant_id": "tenant_12345",
  "clone_from_seed": true
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "pdb_id": "pdb_67890",
    "pdb_name": "PDB_ACME",
    "state": "mounted",
    "created_at": "2025-12-28T14:00:00Z"
  }
}
```

#### Open PDB

```http
POST /api/v1/pdbs/{pdb_id}/open
Authorization: Bearer <token>
Content-Type: application/json

{
  "mode": "READ_WRITE"
}
```

**Modes**: `READ_ONLY`, `READ_WRITE`, `UPGRADE`, `MIGRATE`

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "pdb_id": "pdb_67890",
    "state": "open",
    "open_mode": "READ_WRITE",
    "opened_at": "2025-12-28T14:05:00Z"
  }
}
```

#### Clone PDB

```http
POST /api/v1/pdbs/{pdb_id}/clone
Authorization: Bearer <token>
Content-Type: application/json

{
  "target_name": "PDB_ACME_DEV",
  "clone_type": "snapshot",
  "open_after_clone": true
}
```

**Clone Types**: `hot`, `snapshot`, `metadata_only`

**Response** (202 Accepted):
```json
{
  "success": true,
  "data": {
    "job_id": "clone_job_123",
    "source_pdb": "PDB_ACME",
    "target_pdb": "PDB_ACME_DEV",
    "status": "in_progress",
    "estimated_completion": "2025-12-28T14:30:00Z"
  }
}
```

#### Unplug PDB

```http
POST /api/v1/pdbs/{pdb_id}/unplug
Authorization: Bearer <token>
Content-Type: application/json

{
  "xml_path": "/backups/pdb_acme.xml",
  "keep_datafiles": true
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "pdb_id": "pdb_67890",
    "xml_manifest": "/backups/pdb_acme.xml",
    "state": "unplugged",
    "size_mb": 5120
  }
}
```

### Resource Management

#### Get Resource Usage

```http
GET /api/v1/tenants/{tenant_id}/resources
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "current_usage": {
      "cpu_percent": 45.2,
      "memory_mb": 2048,
      "storage_gb": 150,
      "iops": 850,
      "network_mbps": 75,
      "active_connections": 23
    },
    "quota": {
      "cpu_percent": 200,
      "memory_mb": 4096,
      "storage_gb": 500,
      "iops": 3000,
      "network_mbps": 250,
      "max_connections": 100
    },
    "utilization": {
      "cpu": "22.6%",
      "memory": "50.0%",
      "storage": "30.0%",
      "connections": "23.0%"
    }
  }
}
```

#### Update Resource Quota

```http
PUT /api/v1/tenants/{tenant_id}/quota
Authorization: Bearer <token>
Content-Type: application/json

{
  "cpu_percent": 400,
  "memory_mb": 8192,
  "storage_gb": 1000,
  "iops": 5000
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "quota_updated": true,
    "effective_immediately": true
  }
}
```

---

## GraphQL API

### Types

```graphql
type Tenant {
  tenantId: ID!
  tenantName: String!
  serviceTier: ServiceTier!
  state: TenantState!
  priority: TenantPriority!
  quota: ResourceQuota!
  usage: ResourceUsage!
  sla: SlaMetrics!
  createdAt: DateTime!
}

enum ServiceTier {
  BRONZE
  SILVER
  GOLD
  PLATINUM
}

enum TenantState {
  ACTIVE
  SUSPENDED
  MAINTENANCE
  MIGRATING
  TERMINATED
}

type ResourceQuota {
  cpuPercent: Int!
  memoryMb: Int!
  storageGb: Int!
  iops: Int!
  maxConnections: Int!
}

type ResourceUsage {
  cpuPercent: Float!
  memoryMb: Int!
  storageGb: Int!
  currentIops: Int!
  activeConnections: Int!
  lastUpdated: DateTime!
}

type SlaMetrics {
  uptimePercent: Float!
  avgResponseTimeMs: Float!
  p95ResponseTimeMs: Float!
  errorRatePercent: Float!
  violations: [SlaViolation!]!
}

type PDB {
  pdbId: ID!
  pdbName: String!
  state: PdbState!
  openMode: OpenMode
  tenantId: ID
  createdAt: DateTime!
}

enum PdbState {
  CREATED
  MOUNTED
  OPEN
  CLOSED
  DROPPED
}

enum OpenMode {
  READ_ONLY
  READ_WRITE
  UPGRADE
  MIGRATE
}
```

### Queries

```graphql
type Query {
  # Tenant queries
  tenants(
    page: Int = 1
    pageSize: Int = 20
    tier: ServiceTier
    state: TenantState
  ): TenantConnection!

  tenant(id: ID!): Tenant
  tenantByName(name: String!): Tenant

  # PDB queries
  pdbs(
    page: Int = 1
    pageSize: Int = 20
    state: PdbState
  ): PdbConnection!

  pdb(id: ID!): PDB
  pdbByName(name: String!): PDB

  # Resource queries
  tenantResources(tenantId: ID!): ResourceUsage!
  tenantSla(tenantId: ID!): SlaMetrics!

  # CDB queries
  cdb: CDB!
  cdbStatistics: CdbStatistics!
}
```

### Mutations

```graphql
type Mutation {
  # Tenant management
  createTenant(input: CreateTenantInput!): TenantResult!
  suspendTenant(tenantId: ID!, reason: String!): TenantResult!
  resumeTenant(tenantId: ID!): TenantResult!
  upgradeTier(tenantId: ID!, newTier: ServiceTier!): TenantResult!
  updateQuota(tenantId: ID!, quota: ResourceQuotaInput!): TenantResult!
  terminateTenant(tenantId: ID!): TenantResult!

  # PDB operations
  createPdb(input: CreatePdbInput!): PdbResult!
  openPdb(pdbId: ID!, mode: OpenMode!): PdbResult!
  closePdb(pdbId: ID!): PdbResult!
  clonePdb(input: ClonePdbInput!): CloneJobResult!
  unplugPdb(input: UnplugPdbInput!): PdbResult!
  plugPdb(input: PlugPdbInput!): PdbResult!
  dropPdb(pdbId: ID!, keepDatafiles: Boolean!): PdbResult!
}

input CreateTenantInput {
  tenantName: String!
  adminUser: String!
  adminPassword: String!
  serviceTier: ServiceTier!
  organization: String
}

input CreatePdbInput {
  pdbName: String!
  adminUser: String!
  adminPassword: String!
  tenantId: ID
  cloneFromSeed: Boolean!
}

union TenantResult = Tenant | TenantError

type TenantError {
  message: String!
  code: String!
  details: String
}
```

### Subscriptions

```graphql
type Subscription {
  # Real-time tenant monitoring
  tenantResourceUsage(tenantId: ID!): ResourceUsage!
  tenantSlaViolations(tenantId: ID!): SlaViolation!
  tenantStateChanged(tenantId: ID!): TenantStateChange!

  # PDB events
  pdbStateChanged(pdbId: ID!): PdbStateChange!
  cloneProgress(jobId: ID!): CloneProgress!
}

type TenantStateChange {
  tenantId: ID!
  oldState: TenantState!
  newState: TenantState!
  timestamp: DateTime!
  reason: String
}
```

---

## Resource Management

### Isolation Statistics

#### Memory Isolation

```http
GET /api/v1/isolation/memory?tenant={tenant_id}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "allocated_bytes": 2147483648,
    "quota_bytes": 4294967296,
    "peak_usage_bytes": 3221225472,
    "oom_count": 0,
    "allocation_count": 150000,
    "deallocation_count": 145000
  }
}
```

#### CPU Scheduling

```http
GET /api/v1/isolation/cpu?tenant={tenant_id}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "shares": 1000,
    "min_percent": 10,
    "max_percent": 200,
    "used_cpu_ns": 500000000000,
    "throttled_ns": 5000000000,
    "throttle_ratio": 0.01,
    "allocation_percent": 45.2
  }
}
```

### CDB Operations

#### Get CDB Status

```http
GET /api/v1/cdb/status
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "cdb_name": "CDB_PROD",
    "total_pdbs": 15,
    "max_pdbs": 100,
    "open_pdbs": 12,
    "seed_pdb": "PDB$SEED",
    "uptime_seconds": 864000,
    "shared_services": {
      "undo_tablespace": {
        "size_gb": 2,
        "used_gb": 0.8
      },
      "temp_tablespace": {
        "size_gb": 1,
        "used_gb": 0.3
      }
    }
  }
}
```

---

## Isolation & Security

### Authentication & Authorization

**Headers**:
```http
Authorization: Bearer <jwt_token>
X-Tenant-ID: <tenant_id>
```

**JWT Payload**:
```json
{
  "user_id": "user_123",
  "username": "admin@acme.com",
  "roles": ["tenant_admin", "pdb_manager"],
  "tenant_id": "tenant_12345",
  "permissions": [
    "tenant:read",
    "tenant:write",
    "pdb:manage",
    "resources:update"
  ],
  "exp": 1735382400
}
```

### Permission Model

| Permission | Description |
|------------|-------------|
| `tenant:read` | View tenant information |
| `tenant:write` | Create/update tenants |
| `tenant:admin` | Full tenant management |
| `pdb:read` | View PDB information |
| `pdb:manage` | Create/modify PDBs |
| `pdb:clone` | Clone PDBs |
| `resources:read` | View resource usage |
| `resources:update` | Modify quotas |
| `sla:read` | View SLA metrics |
| `cdb:admin` | CDB administration |

---

## Billing & Metering

### Metering Records

```http
GET /api/v1/tenants/{tenant_id}/metering?start=2025-12-01&end=2025-12-31
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "period": {
      "start": "2025-12-01T00:00:00Z",
      "end": "2025-12-31T23:59:59Z"
    },
    "metrics": {
      "total_cpu_hours": 1440.5,
      "total_memory_gb_hours": 2880.0,
      "total_storage_gb_hours": 3600.0,
      "total_iops": 50000000,
      "total_network_gb": 500.0
    },
    "cost_estimate": {
      "cpu": 288.10,
      "memory": 144.00,
      "storage": 72.00,
      "iops": 25.00,
      "network": 10.00,
      "total": 539.10,
      "currency": "USD"
    }
  }
}
```

### Billing Report

```http
GET /api/v1/tenants/{tenant_id}/billing?period=2025-12
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "tenant_id": "tenant_12345",
    "period": "2025-12",
    "service_tier": "silver",
    "base_fee": 199.00,
    "usage_charges": 340.10,
    "total": 539.10,
    "currency": "USD",
    "breakdown": {
      "compute": 288.10,
      "storage": 72.00,
      "network": 10.00,
      "support": 169.00
    }
  }
}
```

---

## Migration Guide

### Migrating to Multi-Tenant

#### Step 1: Plan Tenant Structure

Identify tenants and map to PDBs:
- 1 PDB per customer (small tenants)
- Multiple PDBs per customer (large tenants)
- Shared PDB for micro-tenants

#### Step 2: Create CDB

```sql
-- Create Container Database
CREATE DATABASE cdb_prod
  ENABLE PLUGGABLE DATABASE;
```

#### Step 3: Create PDB Template

```http
POST /api/v1/pdbs
{
  "pdb_name": "PDB$SEED",
  "admin_user": "pdb_admin",
  "admin_password": "secure_pwd",
  "clone_from_seed": false
}
```

#### Step 4: Migrate Existing Databases

```http
POST /api/v1/pdbs/migrate
{
  "source_database": "old_customer_db",
  "target_pdb": "PDB_CUSTOMER_1",
  "tenant_id": "tenant_12345"
}
```

#### Step 5: Configure Resource Limits

```http
PUT /api/v1/tenants/tenant_12345/quota
{
  "cpu_percent": 200,
  "memory_mb": 4096,
  "storage_gb": 500
}
```

---

## Rate Limiting

Per tenant rate limits based on service tier:

| Tier | Requests/Hour | Burst |
|------|---------------|-------|
| Bronze | 1,000 | 50 |
| Silver | 5,000 | 100 |
| Gold | 25,000 | 250 |
| Platinum | 100,000 | 500 |

**Headers**:
```http
X-RateLimit-Limit: 5000
X-RateLimit-Remaining: 4995
X-RateLimit-Reset: 1735382400
```

---

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `TENANT_NOT_FOUND` | 404 | Tenant does not exist |
| `TENANT_ALREADY_EXISTS` | 409 | Tenant name already taken |
| `QUOTA_EXCEEDED` | 429 | Resource quota exceeded |
| `ISOLATION_VIOLATION` | 403 | Cross-tenant access attempted |
| `INVALID_STATE` | 400 | Invalid state transition |
| `PDB_NOT_FOUND` | 404 | PDB does not exist |
| `INSUFFICIENT_RESOURCES` | 507 | Not enough system resources |
| `SLA_VIOLATION` | 200 | SLA violation detected (warning) |
| `PERMISSION_DENIED` | 403 | Insufficient permissions |

---

## Webhook Events

Register webhooks for tenant events:

```http
POST /api/v1/tenants/{tenant_id}/webhooks
{
  "webhook_url": "https://example.com/webhooks/rustydb",
  "events": [
    "tenant.state.changed",
    "tenant.sla.violated",
    "pdb.opened",
    "quota.exceeded"
  ],
  "secret": "webhook_secret_key"
}
```

**Event Payload**:
```json
{
  "event_id": "evt_123",
  "event_type": "tenant.sla.violated",
  "timestamp": "2025-12-28T16:00:00Z",
  "tenant_id": "tenant_12345",
  "data": {
    "violation_type": "uptime",
    "current_value": 98.5,
    "sla_target": 99.5,
    "severity": "high"
  }
}
```

---

## Best Practices

### Tenant Design

1. **Isolate by Customer**: 1 tenant = 1 customer
2. **Group Micro-Tenants**: Share PDB for very small tenants
3. **Partition Large Tenants**: Multiple PDBs for enterprise customers

### Resource Management

1. **Set Realistic Quotas**: Based on tier and usage patterns
2. **Monitor Utilization**: Track usage vs. quota
3. **Auto-Scale**: Adjust quotas based on demand
4. **Alert on Violations**: Notify before limits are reached

### Performance

1. **Use Connection Pooling**: Pool per tenant
2. **Cache Frequently**: Cache tenant metadata
3. **Batch Operations**: Group operations per tenant
4. **Optimize Queries**: Index tenant_id columns

---

## Additional Resources

- **API Overview**: [API_OVERVIEW.md](./API_OVERVIEW.md)
- **REST API Reference**: [REST_API.md](./REST_API.md)
- **Connection Pool API**: [CONNECTION_POOL.md](./CONNECTION_POOL.md)

---

**Last Updated**: 2025-12-28
**Status**: PROPOSED (Not yet fully implemented)
**Implementation Priority**: HIGH
**Product Version**: RustyDB v0.6.0 Enterprise Server
