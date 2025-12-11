# RustyDB Multitenant API Specification
## Proposed REST & GraphQL APIs for Multitenant Features

**Version**: 1.0.0
**Date**: 2025-12-11
**Status**: PROPOSED (Not yet implemented)

---

## REST API Endpoints

### Tenant Management

#### Create Tenant
```http
POST /api/v1/tenants
Content-Type: application/json

{
  "tenant_name": "acme_corp",
  "admin_user": "admin@acme.com",
  "admin_password": "secure_password",
  "service_tier": "silver",  // bronze, silver, gold, platinum
  "organization": "ACME Corporation",
  "tags": {
    "environment": "production",
    "department": "sales"
  }
}

Response 201:
{
  "tenant_id": "tenant_12345",
  "pdb_id": "PDB_ACME",
  "status": "active",
  "created_at": "2025-12-11T10:00:00Z",
  "resources": {
    "cpu_cores": 2.0,
    "memory_mb": 4096,
    "storage_gb": 100,
    "network_port": 10234
  }
}
```

#### List Tenants
```http
GET /api/v1/tenants?page=1&page_size=20&tier=silver&state=active

Response 200:
{
  "tenants": [
    {
      "tenant_id": "tenant_12345",
      "tenant_name": "acme_corp",
      "service_tier": "silver",
      "state": "active",
      "created_at": "2025-12-11T10:00:00Z"
    }
  ],
  "total": 42,
  "page": 1,
  "page_size": 20
}
```

#### Get Tenant Details
```http
GET /api/v1/tenants/{tenant_id}

Response 200:
{
  "tenant_id": "tenant_12345",
  "tenant_name": "acme_corp",
  "service_tier": "silver",
  "state": "active",
  "priority": "medium",
  "quota": {
    "cpu_percent": 200,
    "memory_mb": 4096,
    "storage_gb": 100,
    "iops": 3000,
    "max_connections": 100
  },
  "usage": {
    "cpu_percent": 45.2,
    "memory_mb": 2048,
    "storage_gb": 35,
    "iops": 850,
    "active_connections": 23
  },
  "sla": {
    "uptime_percent": 99.7,
    "avg_response_time_ms": 28.5,
    "violations": []
  }
}
```

#### Suspend Tenant
```http
POST /api/v1/tenants/{tenant_id}/suspend
Content-Type: application/json

{
  "reason": "Payment overdue",
  "notify_admin": true
}

Response 200:
{
  "tenant_id": "tenant_12345",
  "state": "suspended",
  "suspended_at": "2025-12-11T11:00:00Z",
  "reason": "Payment overdue"
}
```

#### Resume Tenant
```http
POST /api/v1/tenants/{tenant_id}/resume

Response 200:
{
  "tenant_id": "tenant_12345",
  "state": "active",
  "resumed_at": "2025-12-11T12:00:00Z"
}
```

#### Upgrade Service Tier
```http
PUT /api/v1/tenants/{tenant_id}/tier
Content-Type: application/json

{
  "new_tier": "gold",
  "effective_date": "immediate"  // or specific date
}

Response 200:
{
  "tenant_id": "tenant_12345",
  "old_tier": "silver",
  "new_tier": "gold",
  "upgraded_at": "2025-12-11T13:00:00Z",
  "new_quota": {
    "cpu_cores": 4.0,
    "memory_mb": 8192,
    "storage_gb": 250
  }
}
```

### PDB Operations

#### Create Pluggable Database
```http
POST /api/v1/pdbs
Content-Type: application/json

{
  "pdb_name": "PDB_ACME",
  "admin_user": "pdb_admin",
  "admin_password": "secure_pwd",
  "tenant_id": "tenant_12345",
  "clone_from_seed": true
}

Response 201:
{
  "pdb_id": "pdb_67890",
  "pdb_name": "PDB_ACME",
  "state": "mounted",
  "created_at": "2025-12-11T14:00:00Z"
}
```

#### Open PDB
```http
POST /api/v1/pdbs/{pdb_id}/open
Content-Type: application/json

{
  "mode": "READ_WRITE"  // READ_ONLY, READ_WRITE, UPGRADE, MIGRATE
}

Response 200:
{
  "pdb_id": "pdb_67890",
  "state": "open",
  "open_mode": "READ_WRITE",
  "opened_at": "2025-12-11T14:05:00Z"
}
```

#### Clone PDB
```http
POST /api/v1/pdbs/{pdb_id}/clone
Content-Type: application/json

{
  "target_name": "PDB_ACME_DEV",
  "clone_type": "snapshot",  // hot, snapshot, metadata_only
  "open_after_clone": true
}

Response 202:
{
  "job_id": "clone_job_123",
  "source_pdb": "PDB_ACME",
  "target_pdb": "PDB_ACME_DEV",
  "status": "in_progress",
  "estimated_completion": "2025-12-11T14:30:00Z"
}
```

#### Unplug PDB
```http
POST /api/v1/pdbs/{pdb_id}/unplug
Content-Type: application/json

{
  "xml_path": "/backup/pdb_acme.xml",
  "keep_datafiles": true
}

Response 200:
{
  "pdb_id": "pdb_67890",
  "xml_manifest": "/backup/pdb_acme.xml",
  "state": "unplugged",
  "size_mb": 5120
}
```

#### Plug PDB
```http
POST /api/v1/pdbs/plug
Content-Type: application/json

{
  "xml_path": "/backup/pdb_acme.xml",
  "pdb_name": "PDB_ACME_RESTORED",
  "copy_datafiles": true,
  "target_path": "/data/pdbs"
}

Response 201:
{
  "pdb_id": "pdb_99999",
  "pdb_name": "PDB_ACME_RESTORED",
  "state": "mounted",
  "plugged_at": "2025-12-11T15:00:00Z"
}
```

### Resource Management

#### Get Resource Usage
```http
GET /api/v1/tenants/{tenant_id}/resources

Response 200:
{
  "tenant_id": "tenant_12345",
  "current_usage": {
    "cpu_percent": 45.2,
    "memory_mb": 2048,
    "storage_gb": 35,
    "iops": 850,
    "network_mbps": 75,
    "active_connections": 23
  },
  "quota": {
    "cpu_percent": 200,
    "memory_mb": 4096,
    "storage_gb": 100,
    "iops": 3000,
    "network_mbps": 250,
    "max_connections": 100
  },
  "utilization": {
    "cpu": "22.6%",
    "memory": "50.0%",
    "storage": "35.0%",
    "connections": "23.0%"
  }
}
```

#### Update Resource Quota
```http
PUT /api/v1/tenants/{tenant_id}/quota
Content-Type: application/json

{
  "cpu_percent": 400,
  "memory_mb": 8192,
  "storage_gb": 200,
  "iops": 5000
}

Response 200:
{
  "tenant_id": "tenant_12345",
  "quota_updated": true,
  "effective_immediately": true
}
```

### Isolation Statistics

#### Memory Isolation Stats
```http
GET /api/v1/isolation/memory?tenant={tenant_id}

Response 200:
{
  "tenant_id": "tenant_12345",
  "allocated_bytes": 2147483648,
  "quota_bytes": 4294967296,
  "peak_usage_bytes": 3221225472,
  "oom_count": 0,
  "allocation_count": 150000,
  "deallocation_count": 145000
}
```

#### CPU Scheduling Stats
```http
GET /api/v1/isolation/cpu?tenant={tenant_id}

Response 200:
{
  "tenant_id": "tenant_12345",
  "shares": 1000,
  "min_percent": 10,
  "max_percent": 200,
  "used_cpu_ns": 500000000000,
  "throttled_ns": 5000000000,
  "throttle_ratio": 0.01,
  "allocation_percent": 45.2
}
```

### CDB Operations

#### Get CDB Status
```http
GET /api/v1/cdb/status

Response 200:
{
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
  statistics: TenantStatistics!
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

enum TenantPriority {
  CRITICAL
  HIGH
  MEDIUM
  LOW
  BEST_EFFORT
}

type ResourceQuota {
  cpuPercent: Int!
  memoryMb: Int!
  storageGb: Int!
  iops: Int!
  networkMbps: Int!
  maxConnections: Int!
}

type ResourceUsage {
  cpuPercent: Float!
  memoryMb: Int!
  storageGb: Int!
  currentIops: Int!
  networkMbps: Int!
  activeConnections: Int!
  lastUpdated: DateTime!
}

type SlaMetrics {
  uptimePercent: Float!
  avgResponseTimeMs: Float!
  p95ResponseTimeMs: Float!
  p99ResponseTimeMs: Float!
  errorRatePercent: Float!
  violations: [SlaViolation!]!
}

type SlaViolation {
  violationType: String!
  timestamp: DateTime!
  severity: ViolationSeverity!
  description: String!
  remediation: String!
}

enum ViolationSeverity {
  CRITICAL
  HIGH
  MEDIUM
  LOW
}

type PDB {
  pdbId: ID!
  pdbName: String!
  state: PdbState!
  openMode: OpenMode
  tenantId: ID
  datafiles: [String!]!
  tablespaces: [String!]!
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

type CDB {
  cdbName: String!
  totalPdbs: Int!
  maxPdbs: Int!
  openPdbs: Int!
  seedPdb: String!
  uptimeSeconds: Int!
  sharedServices: SharedServices!
}

type SharedServices {
  undoTablespace: TablespaceStats!
  tempTablespace: TablespaceStats!
  commonUsers: [CommonUser!]!
  lockdownProfiles: [LockdownProfile!]!
}
```

### Queries

```graphql
type Query {
  # Tenant queries
  tenants(
    page: Int = 1,
    pageSize: Int = 20,
    tier: ServiceTier,
    state: TenantState
  ): TenantConnection!

  tenant(id: ID!): Tenant

  tenantByName(name: String!): Tenant

  # PDB queries
  pdbs(
    page: Int = 1,
    pageSize: Int = 20,
    state: PdbState
  ): PdbConnection!

  pdb(id: ID!): PDB

  pdbByName(name: String!): PDB

  # CDB queries
  cdb: CDB!

  cdbStatistics: CdbStatistics!

  # Resource queries
  tenantResources(tenantId: ID!): ResourceUsage!

  isolationStats(
    tenantId: ID!,
    type: IsolationType!
  ): IsolationStats!

  # SLA queries
  tenantSla(tenantId: ID!): SlaMetrics!

  # Metering queries
  tenantMeteringRecords(
    tenantId: ID!,
    startDate: DateTime!,
    endDate: DateTime!
  ): [MeteringRecord!]!

  tenantBilling(
    tenantId: ID!,
    period: String!  # "YYYY-MM"
  ): BillingReport!
}

enum IsolationType {
  MEMORY
  CPU
  IO
  NETWORK
  BUFFER_POOL
}

type TenantConnection {
  edges: [TenantEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type TenantEdge {
  node: Tenant!
  cursor: String!
}

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}
```

### Mutations

```graphql
type Mutation {
  # Tenant management
  createTenant(input: CreateTenantInput!): TenantResult!

  suspendTenant(
    tenantId: ID!,
    reason: String!
  ): TenantResult!

  resumeTenant(tenantId: ID!): TenantResult!

  upgradeTier(
    tenantId: ID!,
    newTier: ServiceTier!
  ): TenantResult!

  updatePriority(
    tenantId: ID!,
    priority: TenantPriority!
  ): TenantResult!

  updateQuota(
    tenantId: ID!,
    quota: ResourceQuotaInput!
  ): TenantResult!

  terminateTenant(tenantId: ID!): TenantResult!

  # PDB operations
  createPdb(input: CreatePdbInput!): PdbResult!

  openPdb(
    pdbId: ID!,
    mode: OpenMode!
  ): PdbResult!

  closePdb(
    pdbId: ID!,
    closeMode: CloseMode!
  ): PdbResult!

  clonePdb(input: ClonePdbInput!): CloneJobResult!

  unplugPdb(input: UnplugPdbInput!): PdbResult!

  plugPdb(input: PlugPdbInput!): PdbResult!

  dropPdb(
    pdbId: ID!,
    keepDatafiles: Boolean!
  ): PdbResult!

  # Shared services
  createCommonUser(input: CommonUserInput!): CommonUserResult!

  createLockdownProfile(input: LockdownProfileInput!): LockdownProfileResult!

  applyLockdownProfile(
    pdbId: ID!,
    profileName: String!
  ): PdbResult!
}

input CreateTenantInput {
  tenantName: String!
  adminUser: String!
  adminPassword: String!
  serviceTier: ServiceTier!
  organization: String
  tags: [TagInput!]
}

input TagInput {
  key: String!
  value: String!
}

input CreatePdbInput {
  pdbName: String!
  adminUser: String!
  adminPassword: String!
  tenantId: ID
  cloneFromSeed: Boolean!
}

input ClonePdbInput {
  sourcePdbId: ID!
  targetName: String!
  cloneType: CloneType!
  openAfterClone: Boolean!
}

enum CloneType {
  HOT
  SNAPSHOT
  METADATA_ONLY
}

union TenantResult = Tenant | TenantError

type TenantError {
  message: String!
  code: String!
  details: String
}

union PdbResult = PDB | PdbError

type PdbError {
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

  # System events
  systemAlerts: SystemAlert!
}

type TenantStateChange {
  tenantId: ID!
  oldState: TenantState!
  newState: TenantState!
  timestamp: DateTime!
  reason: String
}

type PdbStateChange {
  pdbId: ID!
  oldState: PdbState!
  newState: PdbState!
  timestamp: DateTime!
}

type CloneProgress {
  jobId: ID!
  progress: Float!  # 0.0 to 1.0
  bytesTransferred: Int!
  estimatedCompletion: DateTime
  status: String!
}
```

---

## Authentication & Authorization

### Headers

All requests must include:

```http
Authorization: Bearer <jwt_token>
X-Tenant-ID: <tenant_id>  # Optional, for tenant-specific operations
```

### JWT Payload

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
  "exp": 1734528000
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
| `common_user:manage` | Manage common users |

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

## Rate Limiting

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 995
X-RateLimit-Reset: 1734528000
```

Per tenant rate limits based on service tier:

| Tier | Requests/Hour | Burst |
|------|---------------|-------|
| Bronze | 1,000 | 50 |
| Silver | 5,000 | 100 |
| Gold | 25,000 | 250 |
| Platinum | 100,000 | 500 |

---

## Webhook Events

Tenants can register webhooks for events:

```json
{
  "webhook_url": "https://example.com/webhooks/rustydb",
  "events": [
    "tenant.state.changed",
    "tenant.sla.violated",
    "pdb.opened",
    "pdb.closed",
    "quota.exceeded",
    "clone.completed"
  ],
  "secret": "webhook_secret_key"
}
```

Event payload example:

```json
{
  "event_id": "evt_123",
  "event_type": "tenant.sla.violated",
  "timestamp": "2025-12-11T16:00:00Z",
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

**Status**: This specification is PROPOSED and not yet implemented in RustyDB.

**Implementation Priority**: HIGH
**Estimated Effort**: 2-3 weeks
**Dependencies**: None (all backend code exists)
