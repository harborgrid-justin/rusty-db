# RustyDB Network & API Layer Documentation
## Version 0.5.1 - Enterprise Release

**Document Version:** 1.0
**Last Updated:** 2025-12-25
**Status:** Enterprise Production Release ($350M)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Network Layer Architecture](#network-layer-architecture)
3. [REST API](#rest-api)
4. [GraphQL API](#graphql-api)
5. [WebSocket Support](#websocket-support)
6. [API Gateway](#api-gateway)
7. [Connection Pooling](#connection-pooling)
8. [Session Management](#session-management)
9. [Security & Authentication](#security--authentication)
10. [Monitoring & Observability](#monitoring--observability)
11. [Client Integration](#client-integration)
12. [Performance & Scalability](#performance--scalability)
13. [Configuration Reference](#configuration-reference)

---

## Executive Summary

RustyDB v0.5.1 provides a comprehensive, enterprise-grade network and API layer supporting multiple protocols and access methods:

### Key Features

- **Multi-Protocol Support**: Binary protocol, REST, GraphQL, WebSocket
- **PostgreSQL Wire Protocol Compatibility**: Compatible with PostgreSQL clients (port 5432)
- **High Performance**: 10,000+ concurrent connections, sub-millisecond latency
- **Enterprise Security**: JWT/OAuth/mTLS authentication, RBAC, rate limiting
- **Real-time Capabilities**: WebSocket subscriptions, GraphQL subscriptions
- **Comprehensive Monitoring**: Prometheus metrics, health checks, distributed tracing
- **Connection Pooling**: DRCP-like pooling with elastic sizing and partitioning
- **API Gateway**: Intelligent routing, security filtering, quota management

### Supported Protocols

| Protocol | Port | Use Case | Performance |
|----------|------|----------|-------------|
| Binary (Bincode) | 5432 | Native RustyDB clients | Highest (binary serialization) |
| REST/HTTP | 8080 | Web applications, microservices | High (JSON over HTTP) |
| GraphQL | 8080/graphql | Modern web apps, mobile | High (flexible queries) |
| WebSocket | 8080/ws | Real-time streaming, subscriptions | Very High (persistent connections) |

---

## Network Layer Architecture

### TCP Server

**Location**: `src/network/server.rs`

The core TCP server provides the foundation for all network communication:

```rust
use rusty_db::network::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new();
    server.run("0.0.0.0:5432").await?;
    Ok(())
}
```

#### Server Configuration

```rust
pub struct Server {
    catalog: Arc<Catalog>,
    txn_manager: Arc<TransactionManager>,
    executor: Arc<Executor>,
    parser: Arc<SqlParser>,
    active_connections: Arc<AtomicUsize>,
}
```

#### Connection Limits

- **Maximum Concurrent Connections**: 10,000
- **Maximum Request Size**: 1 MB (prevents memory exhaustion)
- **Connection Timeout**: Configurable (default: 30s)
- **Idle Timeout**: Configurable (default: 600s)

#### Connection Lifecycle

1. **Accept**: TCP connection accepted
2. **Authenticate**: Credentials validated
3. **Session Creation**: Session context initialized
4. **Request Processing**: Binary protocol requests handled
5. **Connection Cleanup**: Resources released on disconnect

### Wire Protocol

**Location**: `src/network/protocol.rs`

RustyDB uses a binary protocol based on Bincode serialization for maximum performance.

#### Request Types

```rust
pub enum Request {
    /// Execute SQL query
    Query { sql: String },

    /// Begin new transaction
    BeginTransaction,

    /// Commit current transaction
    Commit,

    /// Rollback current transaction
    Rollback,

    /// Health check ping
    Ping,
}
```

#### Response Types

```rust
pub enum Response {
    /// Query execution result
    QueryResult(QueryResult),

    /// Transaction ID
    TransactionId(u64),

    /// Success acknowledgment
    Ok,

    /// Error message
    Error(String),

    /// Ping response
    Pong,
}
```

#### Security Constraints

- **Maximum SQL Length**: 1 MB (prevents memory exhaustion)
- **Maximum Bincode Size**: 16 MB (prevents DoS attacks)
- **Request Validation**: All requests validated before processing

### Advanced Protocol Features

**Location**: `src/network/advanced_protocol/`

The advanced protocol layer provides enterprise features:

#### Protocol Capabilities

- **Multi-Version Support**: Protocol negotiation for backward compatibility
- **Compression**: LZ4, Snappy, Zstd compression algorithms
- **Streaming**: Large result set streaming with flow control
- **Pipelining**: Request batching and pipelining
- **Circuit Breaker**: Automatic failure detection and recovery

#### Message Types

```rust
pub enum MessageType {
    Query,
    QueryResponse,
    PreparedStatement,
    BulkInsert,
    Streaming,
    Heartbeat,
    Notification,
    Handshake,
}
```

#### Protocol Extensions

```rust
pub struct ProtocolManager {
    version: ProtocolVersion,
    extensions: ExtensionRegistry,
    codec: WireCodec,
    flow_control: FlowControlManager,
    circuit_breaker: CircuitBreaker,
    rate_limiter: RateLimiter,
}
```

### Cluster Networking

**Location**: `src/network/cluster_network/`

Enterprise clustering with high availability and load balancing:

#### Cluster Features

- **SWIM Protocol**: Scalable membership management
- **Gossip Protocol**: Efficient state dissemination
- **Raft Consensus**: Leader election for distributed coordination
- **Load Balancing**: Round-robin, least-connections, locality-aware
- **Automatic Failover**: Health monitoring and session migration

#### Routing Strategies

```rust
pub enum RoutingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    LocalityAware,
    ConsistentHashing,
    LatencyBased,
}
```

#### Node Health Monitoring

```rust
pub struct NetworkHealthMonitor {
    latency_tracker: LatencyTracker,
    packet_loss_detector: PacketLossDetector,
    bandwidth_monitor: BandwidthMonitor,
    route_optimizer: RouteOptimizer,
}
```

### Port Management

**Location**: `src/network/ports/`

Dynamic port allocation and management for distributed deployments:

#### Port Configuration

```rust
pub struct PortConfig {
    pub base_port: u16,              // Default: 5432
    pub port_range_start: u16,       // Default: 6000
    pub port_range_end: u16,         // Default: 7000
    pub enable_ipv6: bool,           // Default: true
    pub enable_unix_sockets: bool,   // Default: true
    pub enable_nat_traversal: bool,  // Default: false
    pub bind_addresses: Vec<String>,
}
```

#### Allocation Strategies

- **Sequential**: Simple incremental allocation
- **Random**: Random port selection from range
- **Hash-Based**: Consistent hashing for service affinity

#### NAT Traversal

- **STUN**: Session Traversal Utilities for NAT
- **UPnP**: Universal Plug and Play port mapping
- **NAT-PMP**: NAT Port Mapping Protocol
- **ICE-lite**: Interactive Connectivity Establishment

---

## REST API

**Location**: `src/api/rest/`

Enterprise REST API built on Axum framework with OpenAPI documentation.

### API Server

```rust
use rusty_db::api::{RestApiServer, ApiConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ApiConfig::default();
    let server = RestApiServer::new(config).await?;
    server.run("0.0.0.0:8080").await?;
    Ok(())
}
```

### Configuration

```rust
pub struct ApiConfig {
    pub listen_addr: String,              // Default: "0.0.0.0"
    pub port: u16,                        // Default: 8080
    pub enable_cors: bool,                // Default: true
    pub cors_origins: Vec<String>,        // Allowed origins
    pub rate_limit_rps: u64,              // Default: 100
    pub request_timeout_secs: u64,        // Default: 30
    pub max_body_size: usize,             // Default: 10 MB
    pub enable_swagger: bool,             // Default: true
    pub enable_auth: bool,                // Default: false
    pub max_connections: usize,           // Default: 1000
    pub default_page_size: usize,         // Default: 50
    pub max_page_size: usize,             // Default: 1000
    pub graphql_max_depth: usize,         // Default: 10
    pub graphql_max_complexity: usize,    // Default: 1000
}
```

### Core Endpoints

#### Database Operations

```http
# Execute SQL Query
POST /api/v1/query
Content-Type: application/json

{
  "sql": "SELECT * FROM users WHERE active = true",
  "params": []
}

# Batch Execute
POST /api/v1/batch
Content-Type: application/json

{
  "statements": [
    "INSERT INTO users (name) VALUES ('Alice')",
    "INSERT INTO users (name) VALUES ('Bob')"
  ]
}
```

#### Transaction Management

```http
# Begin Transaction
POST /api/v1/transactions
Response: { "transaction_id": 12345 }

# Commit Transaction
POST /api/v1/transactions/{id}/commit

# Rollback Transaction
POST /api/v1/transactions/{id}/rollback

# Create Savepoint
POST /api/v1/transactions/{id}/savepoints
{ "name": "sp1" }

# Rollback to Savepoint
POST /api/v1/transactions/{id}/savepoints/{name}/rollback
```

#### Table Management

```http
# List Tables
GET /api/v1/tables?schema=public

# Get Table Details
GET /api/v1/tables/{table_name}

# Create Table
POST /api/v1/tables
{
  "name": "products",
  "columns": [
    { "name": "id", "type": "INTEGER", "primary_key": true },
    { "name": "name", "type": "VARCHAR(255)" },
    { "name": "price", "type": "DECIMAL(10,2)" }
  ]
}

# Update Table
PUT /api/v1/tables/{table_name}

# Delete Table
DELETE /api/v1/tables/{table_name}
```

#### Schema Management

```http
# Get Schema
GET /api/v1/schema/{schema_name}

# List Schemas
GET /api/v1/schemas
```

### Administrative Endpoints

#### User Management

```http
# List Users
GET /api/v1/admin/users

# Create User
POST /api/v1/admin/users
{
  "username": "john_doe",
  "password": "secure_password",
  "email": "john@example.com",
  "roles": ["user", "analyst"]
}

# Get User
GET /api/v1/admin/users/{username}

# Update User
PUT /api/v1/admin/users/{username}

# Delete User
DELETE /api/v1/admin/users/{username}
```

#### Role Management

```http
# List Roles
GET /api/v1/admin/roles

# Create Role
POST /api/v1/admin/roles
{
  "name": "analyst",
  "permissions": ["read:tables", "execute:queries"]
}

# Get Role
GET /api/v1/admin/roles/{role_name}

# Update Role
PUT /api/v1/admin/roles/{role_name}

# Delete Role
DELETE /api/v1/admin/roles/{role_name}
```

#### Configuration

```http
# Get Configuration
GET /api/v1/admin/config

# Update Configuration
PUT /api/v1/admin/config
{
  "max_connections": 500,
  "query_timeout": 60000
}
```

#### Backup & Maintenance

```http
# Create Backup
POST /api/v1/admin/backup
{
  "type": "full",
  "destination": "/backups/backup_20251225.db"
}

# Run Maintenance
POST /api/v1/admin/maintenance
{
  "operation": "vacuum",
  "options": { "analyze": true }
}
```

### Monitoring Endpoints

```http
# Get Metrics
GET /api/v1/monitoring/metrics

# Prometheus Format
GET /api/v1/monitoring/prometheus

# Get Alerts
GET /api/v1/monitoring/alerts

# Acknowledge Alert
POST /api/v1/monitoring/alerts/{id}/acknowledge

# Get Logs
GET /api/v1/monitoring/logs?level=error&limit=100

# Performance Data
GET /api/v1/monitoring/performance

# Query Statistics
GET /api/v1/monitoring/query-stats

# Session Statistics
GET /api/v1/monitoring/session-stats
```

### Health Checks

```http
# Liveness Probe
GET /health/live

# Readiness Probe
GET /health/ready

# Startup Probe
GET /health/startup

# Full Health Check
GET /api/v1/health
Response:
{
  "status": "healthy",
  "checks": {
    "database": "ok",
    "storage": "ok",
    "replication": "ok",
    "cluster": "ok"
  },
  "uptime_seconds": 86400,
  "version": "0.5.1"
}
```

### OpenAPI Documentation

Interactive API documentation available at:

- **Swagger UI**: `http://localhost:8080/swagger-ui/`
- **OpenAPI JSON**: `http://localhost:8080/api/v1/openapi.json`
- **ReDoc**: `http://localhost:8080/redoc/`

---

## GraphQL API

**Location**: `src/api/graphql/`

Modern GraphQL API with real-time subscriptions, built on `async-graphql`.

### GraphQL Endpoint

```http
POST /graphql
Content-Type: application/json

{
  "query": "query { tables { name columns { name type } } }"
}
```

### GraphQL Playground

Interactive GraphQL IDE available at:
- **Playground**: `http://localhost:8080/graphql` (GET request)

### Schema Overview

```graphql
type Query {
  # Schema Operations
  schemas: [DatabaseSchema!]!
  schema(name: String!): DatabaseSchema

  # Table Operations
  tables(schema: String, limit: Int, offset: Int): [TableType!]!
  table(name: String!, schema: String): TableType

  # Query Operations
  queryTable(
    table: String!
    whereClause: WhereClause
    orderBy: [OrderBy!]
    limit: Int
    offset: Int
  ): QueryResult!

  queryTables(
    tables: [String!]!
    joins: [JoinInput!]
    whereClause: WhereClause
    orderBy: [OrderBy!]
    limit: Int
  ): QueryResult!

  # Pagination
  queryTableConnection(
    table: String!
    whereClause: WhereClause
    orderBy: [OrderBy!]
    first: Int
    after: String
    last: Int
    before: String
  ): RowConnection!

  # Single Row
  getRow(table: String!, id: ID!): RowType

  # Aggregations
  aggregate(
    table: String!
    aggregates: [AggregateInput!]!
    whereClause: WhereClause
    groupBy: [String!]
  ): [AggregateResult!]!

  # Search
  search(
    table: String!
    query: String!
    fields: [String!]
    limit: Int
  ): SearchResult!
}

type Mutation {
  # Insert Operations
  insertOne(table: String!, data: JSON!): MutationResult!
  insertMany(table: String!, data: [JSON!]!): MutationResult!

  # Update Operations
  updateOne(table: String!, id: ID!, data: JSON!): MutationResult!
  updateMany(table: String!, whereClause: WhereClause!, data: JSON!): MutationResult!

  # Delete Operations
  deleteOne(table: String!, id: ID!): MutationResult!
  deleteMany(table: String!, whereClause: WhereClause!): MutationResult!

  # Upsert Operations
  upsert(table: String!, data: JSON!, conflictColumns: [String!]!): MutationResult!

  # Transaction Operations
  executeTransaction(operations: [TransactionOperation!]!): TransactionResult!
}

type Subscription {
  # Table Change Subscriptions
  tableChanges(table: String!, whereClause: WhereClause): TableChange!

  # Row Operations
  rowInserted(table: String!, whereClause: WhereClause): RowInserted!
  rowUpdated(table: String!, whereClause: WhereClause): RowUpdated!
  rowDeleted(table: String!, whereClause: WhereClause): RowDeleted!
  rowChanges(table: String!, id: ID!): RowChange!

  # Aggregations
  aggregateChanges(
    table: String!
    aggregates: [AggregateInput!]!
    whereClause: WhereClause
    intervalSeconds: Int
  ): AggregateChange!

  # Query Changes
  queryChanges(
    table: String!
    whereClause: WhereClause
    orderBy: [OrderBy!]
  ): QueryChange!
}
```

### Query Examples

#### Basic Table Query

```graphql
query GetUsers {
  queryTable(
    table: "users"
    whereClause: {
      field: "active"
      op: EQUALS
      value: true
    }
    orderBy: [{ field: "created_at", order: DESC }]
    limit: 10
  ) {
    ... on QuerySuccess {
      rows {
        id
        data
      }
      totalCount
      executionTimeMs
      hasMore
    }
    ... on QueryError {
      message
      code
    }
  }
}
```

#### Join Query

```graphql
query GetOrdersWithCustomers {
  queryTables(
    tables: ["orders", "customers"]
    joins: [{
      table: "customers"
      type: INNER
      on: {
        leftField: "customer_id"
        rightField: "id"
      }
    }]
    limit: 20
  ) {
    ... on QuerySuccess {
      rows {
        id
        data
      }
    }
  }
}
```

#### Aggregation Query

```graphql
query SalesSummary {
  aggregate(
    table: "sales"
    aggregates: [
      { function: SUM, field: "amount", alias: "total_sales" }
      { function: COUNT, field: "*", alias: "num_transactions" }
      { function: AVG, field: "amount", alias: "avg_sale" }
    ]
    groupBy: ["product_category"]
  ) {
    group
    results {
      alias
      value
    }
  }
}
```

#### Cursor-Based Pagination

```graphql
query PaginatedUsers {
  queryTableConnection(
    table: "users"
    first: 20
    after: "cursor_token_here"
  ) {
    edges {
      node {
        id
        data
      }
      cursor
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
    totalCount
  }
}
```

### Mutation Examples

#### Insert Single Row

```graphql
mutation CreateUser {
  insertOne(
    table: "users"
    data: {
      name: "Alice Johnson"
      email: "alice@example.com"
      age: 30
    }
  ) {
    ... on MutationSuccess {
      affectedRows
      returning {
        id
        data
      }
      executionTimeMs
    }
    ... on MutationError {
      message
      code
    }
  }
}
```

#### Batch Insert

```graphql
mutation CreateUsers {
  insertMany(
    table: "users"
    data: [
      { name: "Bob Smith", email: "bob@example.com" }
      { name: "Carol White", email: "carol@example.com" }
    ]
  ) {
    ... on MutationSuccess {
      affectedRows
      returning {
        id
        data
      }
    }
  }
}
```

#### Update with Condition

```graphql
mutation UpdateInactiveUsers {
  updateMany(
    table: "users"
    whereClause: {
      field: "last_login"
      op: LESS_THAN
      value: "2024-01-01"
    }
    data: { active: false }
  ) {
    ... on MutationSuccess {
      affectedRows
    }
  }
}
```

#### Transaction

```graphql
mutation TransferFunds {
  executeTransaction(
    operations: [
      {
        type: UPDATE
        table: "accounts"
        whereClause: { field: "id", op: EQUALS, value: 1 }
        data: { balance: { decrement: 100 } }
      }
      {
        type: UPDATE
        table: "accounts"
        whereClause: { field: "id", op: EQUALS, value: 2 }
        data: { balance: { increment: 100 } }
      }
    ]
  ) {
    ... on TransactionExecutionResult {
      success
      affectedRows
      executionTimeMs
    }
  }
}
```

### Subscription Examples

#### Real-Time Table Changes

```graphql
subscription WatchUsers {
  tableChanges(table: "users") {
    changeType
    table
    row {
      id
      data
    }
    timestamp
  }
}
```

#### Filtered Inserts

```graphql
subscription NewPremiumUsers {
  rowInserted(
    table: "users"
    whereClause: {
      field: "subscription_tier"
      op: EQUALS
      value: "premium"
    }
  ) {
    row {
      id
      data
    }
    timestamp
  }
}
```

#### Aggregation Monitoring

```graphql
subscription LiveSalesMetrics {
  aggregateChanges(
    table: "sales"
    aggregates: [
      { function: SUM, field: "amount", alias: "total" }
      { function: COUNT, field: "*", alias: "count" }
    ]
    intervalSeconds: 5
  ) {
    table
    results {
      alias
      value
    }
    timestamp
  }
}
```

### GraphQL Configuration

```rust
use rusty_db::api::graphql::SchemaConfig;

let config = SchemaConfig {
    max_depth: Some(10),              // Query depth limit
    max_complexity: Some(1000),       // Query complexity limit
    enable_performance_extension: true,
    enable_tracing: false,
    enable_introspection: false,      // Disable in production
    enable_playground: false,         // Disable in production
};

let schema = build_schema_with_config(config);
```

### Security Features

- **Query Depth Limiting**: Prevents deeply nested queries (DoS protection)
- **Complexity Analysis**: Limits computational complexity per query
- **Introspection Control**: Disable schema introspection in production
- **Field-Level Authorization**: Check permissions per field
- **Rate Limiting**: Per-user query rate limits
- **Persisted Queries**: Only allow pre-approved queries in production

---

## WebSocket Support

**Location**: `src/api/graphql/websocket_transport.rs`, `src/api/rest/handlers/websocket_handlers.rs`

Enterprise WebSocket support for real-time data streaming.

### GraphQL WebSocket Protocol

RustyDB implements the `graphql-ws` protocol for GraphQL subscriptions.

#### Connection Establishment

```javascript
const ws = new WebSocket('ws://localhost:8080/graphql');

// Connection initialization
ws.send(JSON.stringify({
  type: 'connection_init',
  payload: {
    authorization: 'Bearer <token>'
  }
}));

// Subscribe to data changes
ws.send(JSON.stringify({
  type: 'subscribe',
  id: '1',
  payload: {
    query: `
      subscription {
        tableChanges(table: "users") {
          changeType
          row { id data }
        }
      }
    `
  }
}));
```

#### Message Types

```rust
pub enum GraphQLWsMessage {
    // Client -> Server: Initialize connection
    ConnectionInit { payload: Option<ConnectionInitPayload> },

    // Server -> Client: Connection acknowledged
    ConnectionAck { payload: Option<Value> },

    // Bidirectional: Heartbeat
    Ping { payload: Option<Value> },
    Pong { payload: Option<Value> },

    // Client -> Server: Start operation
    Subscribe { id: String, payload: SubscribePayload },

    // Server -> Client: Data
    Next { id: String, payload: Value },

    // Server -> Client: Error
    Error { id: String, payload: Vec<GraphQLError> },

    // Bidirectional: Complete operation
    Complete { id: String },
}
```

### REST WebSocket Endpoints

```http
# Query Streaming
GET /ws/query-stream?sql=SELECT+*+FROM+users

# Metrics Streaming
GET /ws/metrics-stream?interval=1000

# Events Streaming
GET /ws/events-stream?types=insert,update,delete

# Replication Streaming
GET /ws/replication-stream
```

### WebSocket Management

```http
# Get WebSocket Status
GET /api/v1/websocket/status

# List Active Connections
GET /api/v1/websocket/connections

# Get Connection Details
GET /api/v1/websocket/connections/{connection_id}

# Disconnect Connection
DELETE /api/v1/websocket/connections/{connection_id}

# Broadcast Message
POST /api/v1/websocket/broadcast
{
  "message": { "type": "notification", "data": "..." }
}

# List Subscriptions
GET /api/v1/websocket/subscriptions

# Create Subscription
POST /api/v1/websocket/subscriptions
{
  "type": "table_changes",
  "table": "orders",
  "filter": { "status": "pending" }
}

# Delete Subscription
DELETE /api/v1/websocket/subscriptions/{subscription_id}
```

### Client Examples

#### JavaScript/TypeScript

```javascript
import { createClient } from 'graphql-ws';

const client = createClient({
  url: 'ws://localhost:8080/graphql',
  connectionParams: {
    authorization: 'Bearer <token>',
  },
});

// Subscribe to real-time updates
const unsubscribe = client.subscribe(
  {
    query: `
      subscription {
        tableChanges(table: "orders") {
          changeType
          row { id data }
        }
      }
    `,
  },
  {
    next: (data) => console.log('New data:', data),
    error: (error) => console.error('Error:', error),
    complete: () => console.log('Complete'),
  }
);

// Cleanup
unsubscribe();
```

#### Python

```python
import asyncio
import websockets
import json

async def subscribe_to_changes():
    uri = "ws://localhost:8080/graphql"

    async with websockets.connect(uri) as websocket:
        # Initialize connection
        await websocket.send(json.dumps({
            "type": "connection_init",
            "payload": {"authorization": "Bearer <token>"}
        }))

        # Wait for ack
        response = await websocket.recv()
        print(f"Connected: {response}")

        # Subscribe
        await websocket.send(json.dumps({
            "type": "subscribe",
            "id": "1",
            "payload": {
                "query": """
                    subscription {
                        tableChanges(table: "users") {
                            changeType
                            row { id data }
                        }
                    }
                """
            }
        }))

        # Receive updates
        while True:
            message = await websocket.recv()
            data = json.loads(message)
            print(f"Update: {data}")

asyncio.run(subscribe_to_changes())
```

---

## API Gateway

**Location**: `src/api/gateway/`

Enterprise API Gateway with intelligent routing, security, and traffic management.

### Gateway Architecture

```rust
pub struct ApiGateway {
    config: Arc<RwLock<GatewayConfig>>,
    routes: Arc<RwLock<HashMap<String, Route>>>,
    auth_manager: Arc<AuthenticationManager>,
    authz_engine: Arc<AuthorizationEngine>,
    rate_limiter: Arc<RateLimiter>,
    security_filter: Arc<SecurityFilter>,
    service_registry: Arc<RwLock<ServiceRegistry>>,
    metrics: Arc<RwLock<GatewayMetrics>>,
    audit_logger: Arc<Mutex<AuditLogger>>,
}
```

### Gateway Configuration

```rust
pub struct GatewayConfig {
    pub listen_addr: String,
    pub listen_port: u16,
    pub enable_ssl: bool,
    pub ssl_cert_path: Option<String>,
    pub ssl_key_path: Option<String>,
    pub enable_mtls: bool,
    pub max_request_size: usize,
    pub request_timeout: Duration,
    pub enable_rate_limiting: bool,
    pub enable_circuit_breaker: bool,
    pub enable_load_balancing: bool,
    pub health_check_interval: Duration,
}
```

### Route Configuration

```rust
pub struct Route {
    pub id: String,
    pub path_pattern: String,
    pub methods: Vec<String>,
    pub backend_service: BackendService,
    pub auth_required: bool,
    pub required_permissions: Vec<String>,
    pub rate_limit: Option<RateLimitConfig>,
    pub timeout: Duration,
    pub retry_policy: Option<RetryPolicy>,
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}
```

### Authentication Methods

#### JWT (JSON Web Tokens)

```rust
pub struct JwtValidator {
    signing_keys: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    issuer: String,
    audience: Vec<String>,
    algorithm: JwtAlgorithm,  // HS256, RS256, ES256, etc.
    expiration_tolerance: u64,
}
```

**Example JWT Request:**

```http
POST /api/v1/query
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{ "sql": "SELECT * FROM users" }
```

#### OAuth 2.0 / OpenID Connect

```rust
pub struct OAuthProvider {
    config: Arc<RwLock<OAuthConfig>>,
    token_cache: Arc<RwLock<HashMap<String, OAuthToken>>>,
}

pub struct OAuthConfig {
    pub auth_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: Option<String>,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub discovery_url: Option<String>,
}
```

#### API Keys

```rust
pub struct ApiKeyStore {
    keys: HashMap<String, ApiKeyMetadata>,
}

pub struct ApiKeyMetadata {
    pub key_id: String,
    pub user_id: String,
    pub created_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub permissions: Vec<String>,
    pub rate_limit: Option<RateLimitConfig>,
    pub last_used: Option<SystemTime>,
    pub usage_count: u64,
}
```

**Example API Key Request:**

```http
GET /api/v1/tables
X-API-Key: sk_live_abc123xyz789...
```

#### mTLS (Mutual TLS)

```rust
pub struct MtlsValidator {
    ca_certs: Arc<RwLock<Vec<Vec<u8>>>>,
    require_client_cert: bool,
    verify_depth: u32,
    allowed_common_names: Option<Vec<String>>,
}
```

### Rate Limiting

#### Algorithms

```rust
pub enum RateLimitType {
    TokenBucket,     // Smooth rate limiting with burst
    SlidingWindow,   // Precise time-based limiting
    FixedWindow,     // Simple time bucket limiting
}
```

#### Configuration

```rust
pub struct RateLimitConfig {
    pub limit_type: RateLimitType,
    pub requests: u64,        // Requests allowed
    pub window: u64,          // Time window (seconds)
    pub burst: Option<u64>,   // Burst capacity
}
```

#### Usage Examples

**Per-User Rate Limiting:**

```rust
let config = RateLimitConfig {
    limit_type: RateLimitType::TokenBucket,
    requests: 100,      // 100 requests
    window: 60,         // per minute
    burst: Some(120),   // burst up to 120
};

gateway.configure_rate_limit("user:alice", config);
```

**Per-Route Rate Limiting:**

```rust
let route = Route {
    id: "query_endpoint".to_string(),
    path_pattern: "/api/v1/query".to_string(),
    rate_limit: Some(RateLimitConfig {
        limit_type: RateLimitType::SlidingWindow,
        requests: 1000,
        window: 60,
        burst: None,
    }),
    // ... other config
};
```

### Authorization

```rust
pub struct AuthorizationEngine {
    policies: Arc<RwLock<HashMap<String, AuthorizationPolicy>>>,
    role_permissions: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

pub struct AuthorizationPolicy {
    pub resource: String,
    pub actions: Vec<String>,
    pub effect: Effect,      // Allow or Deny
    pub conditions: Vec<Condition>,
}
```

**Permission Examples:**

```rust
// User permissions
let permissions = vec![
    "read:tables",
    "execute:queries",
    "create:tables",
    "delete:tables",
];

// Check authorization
authz_engine.authorize(&session, &["read:tables"])?;
```

### Security Filtering

```rust
pub struct SecurityFilter {
    sql_injection_detector: SqlInjectionDetector,
    xss_detector: XssDetector,
    request_validator: RequestValidator,
    ip_blacklist: Arc<RwLock<HashSet<IpAddr>>>,
    ip_whitelist: Arc<RwLock<HashSet<IpAddr>>>,
}
```

**Security Checks:**

- SQL Injection Detection
- XSS (Cross-Site Scripting) Detection
- CSRF Token Validation
- IP Whitelisting/Blacklisting
- Request Size Validation
- Header Validation

### Load Balancing

```rust
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    IpHash,
    Random,
}

pub struct BackendService {
    pub service_id: String,
    pub endpoints: Vec<String>,
    pub load_balancing: LoadBalancingStrategy,
    pub health_check_url: Option<String>,
    pub timeout: Duration,
}
```

### Circuit Breaker

```rust
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
}

pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing recovery
}
```

**Configuration:**

```rust
let circuit_breaker = CircuitBreakerConfig {
    failure_threshold: 5,      // Open after 5 failures
    success_threshold: 2,      // Close after 2 successes
    timeout: Duration::from_secs(30),
};
```

### Audit Logging

```rust
pub struct AuditLogger {
    logs: Vec<AuditLog>,
    destination: LogDestination,
}

pub struct AuditLog {
    pub timestamp: SystemTime,
    pub request_id: String,
    pub user_id: Option<String>,
    pub client_ip: IpAddr,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub error: Option<String>,
}
```

### Metrics

```rust
pub struct GatewayMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub auth_failures: u64,
    pub authz_failures: u64,
    pub rate_limit_rejections: u64,
    pub security_blocks: u64,
    pub requests_by_route: HashMap<String, u64>,
    pub requests_by_protocol: HashMap<Protocol, u64>,
    pub avg_latency_ms: f64,
}
```

---

## Connection Pooling

**Location**: `src/pool/connection_pool.rs`

Enterprise-grade connection pooling inspired by Oracle's DRCP (Database Resident Connection Pooling).

### Pool Architecture

```rust
pub struct ConnectionPool<T> {
    config: PoolConfig,
    factory: Arc<dyn ConnectionFactory<T>>,
    idle_connections: Arc<RwLock<VecDeque<PooledConnection<T>>>>,
    active_connections: Arc<RwLock<HashMap<u64, PooledConnection<T>>>>,
    wait_queue: Arc<WaitQueue>,
    statistics: Arc<PoolStatistics>,
    partitions: Option<Arc<PartitionManager<T>>>,
}
```

### Pool Configuration

```rust
pub struct PoolConfig {
    // Core sizing
    pub min_size: usize,              // Default: 5
    pub max_size: usize,              // Default: 100
    pub initial_size: usize,          // Default: 10

    // Timeouts
    pub acquire_timeout: Duration,    // Default: 30s
    pub max_lifetime: Option<Duration>,    // Default: 3600s
    pub idle_timeout: Option<Duration>,    // Default: 600s

    // Validation
    pub validate_on_acquire: bool,    // Default: true
    pub validate_on_release: bool,    // Default: false
    pub validation_timeout: Duration, // Default: 5s

    // Wait queue
    pub max_wait_queue_size: usize,   // Default: 1000
    pub fair_queue: bool,             // Default: true

    // Performance
    pub creation_throttle: Option<u64>,       // Default: 10/s
    pub maintenance_interval: Duration,       // Default: 30s
    pub statement_cache_size: usize,          // Default: 100

    // Monitoring
    pub leak_detection_threshold: Option<Duration>,  // Default: 300s

    // Advanced
    pub enable_partitioning: bool,    // Default: false
}
```

### Pool Usage

#### Basic Usage

```rust
use rusty_db::pool::{ConnectionPool, PoolConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure pool
    let config = PoolConfig::builder()
        .min_size(10)
        .max_size(100)
        .acquire_timeout(Duration::from_secs(30))
        .build()?;

    // Create pool with factory
    let pool = ConnectionPool::new(config, factory).await?;

    // Acquire connection
    let conn = pool.acquire().await?;

    // Use connection
    conn.execute("SELECT * FROM users").await?;

    // Connection automatically returned to pool on drop

    Ok(())
}
```

#### Builder Pattern

```rust
let config = PoolConfig::builder()
    .min_size(5)
    .max_size(50)
    .initial_size(10)
    .acquire_timeout(Duration::from_secs(10))
    .max_lifetime(Duration::from_secs(3600))
    .idle_timeout(Duration::from_secs(300))
    .statement_cache_size(100)
    .enable_partitioning(true)
    .build()?;
```

### Pool Features

#### Elastic Sizing

The pool automatically adjusts between min_size and max_size based on demand:

- **Scale Up**: Create new connections when pool is exhausted
- **Scale Down**: Close idle connections exceeding min_size
- **Throttling**: Limit connection creation rate to prevent thundering herd

#### Connection Lifecycle

```rust
pub enum ConnectionState {
    Idle,           // Available in pool
    Active,         // In use by client
    Validation,     // Being validated
    Creating,       // Being created
    Closed,         // Permanently closed
}
```

**Lifecycle Events:**

1. **Creation**: Factory creates new connection
2. **Validation**: Health check before use
3. **Acquisition**: Handed to client
4. **Release**: Returned to pool
5. **Recycling**: State reset or replacement
6. **Retirement**: Closed after max lifetime

#### Statement Caching

Each pooled connection maintains a prepared statement cache:

```rust
pub struct StatementCache {
    cache: HashMap<String, PreparedStatement>,
    max_size: usize,
    hits: u64,
    misses: u64,
}
```

**Benefits:**
- Reduced parsing overhead
- Improved query performance
- LRU eviction policy

#### Wait Queue Management

```rust
pub struct WaitQueue {
    queue: VecDeque<Waiter>,
    max_size: usize,
    fair_mode: bool,           // FIFO vs Priority
    starvation_threshold: Duration,
    deadlock_detector: DeadlockDetector,
}
```

**Queue Strategies:**

- **Fair (FIFO)**: First-in-first-out ordering
- **Priority**: High-priority requests served first
- **Starvation Prevention**: Boost waiting requests
- **Deadlock Detection**: Identify circular waits

#### Pool Partitioning

Isolate connections by user, application, or tenant:

```rust
pub enum PartitionType {
    ByUser,
    ByApplication,
    ByTenant,
    ByService,
    Custom(String),
}

pub struct PartitionLimits {
    pub min_connections: usize,
    pub max_connections: usize,
    pub max_wait_queue: usize,
}
```

**Example:**

```rust
let partition = PartitionRequest {
    partition_type: PartitionType::ByUser,
    partition_key: "user:alice".to_string(),
    limits: PartitionLimits {
        min_connections: 2,
        max_connections: 10,
        max_wait_queue: 100,
    },
};

pool.create_partition(partition).await?;
```

#### Leak Detection

Automatically detect connections not returned to pool:

```rust
pub struct LeakDetector {
    tracked_connections: HashMap<u64, LeakInfo>,
    threshold: Duration,
}

pub struct LeakInfo {
    pub connection_id: u64,
    pub acquired_at: Instant,
    pub stack_trace: Option<String>,
    pub user: Option<String>,
}
```

**Configuration:**

```rust
let config = PoolConfig::builder()
    .leak_detection_threshold(Some(Duration::from_secs(300)))
    .build()?;
```

### Pool Monitoring

#### Statistics

```rust
pub struct PoolStatistics {
    // Connection counts
    pub total_connections: usize,
    pub idle_connections: usize,
    pub active_connections: usize,

    // Acquisition metrics
    pub total_acquisitions: u64,
    pub successful_acquisitions: u64,
    pub failed_acquisitions: u64,
    pub timeout_acquisitions: u64,

    // Timing metrics
    pub avg_acquire_time_ms: f64,
    pub max_acquire_time_ms: f64,
    pub avg_connection_lifetime_ms: f64,

    // Wait queue
    pub current_waiters: usize,
    pub max_waiters: usize,

    // Health
    pub connection_errors: u64,
    pub validation_failures: u64,
    pub leaked_connections: u64,
}
```

#### Metrics Export

```rust
// Get current statistics
let stats = pool.get_statistics().await;

// Export in various formats
let prometheus = pool.export_metrics(ExportFormat::Prometheus).await?;
let json = pool.export_metrics(ExportFormat::Json).await?;
```

### Recycling Strategies

```rust
pub enum RecyclingStrategy {
    Fast,       // Quick validation and state reset
    Checked,    // Full reset including session state
    Replace,    // Create new connection
    Adaptive,   // Age-based strategy selection
}
```

**Configuration:**

```rust
pool.set_recycling_strategy(RecyclingStrategy::Adaptive).await;
```

---

## Session Management

**Location**: `src/pool/session_manager.rs`, `src/pool/sessions/`

Enterprise session management with Oracle-like capabilities.

### Session Architecture

```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
    auth_provider: Arc<dyn AuthenticationProvider>,
    resource_controller: Arc<ResourceController>,
    event_manager: Arc<SessionEventManager>,
    pool: Arc<SessionPool>,
}
```

### Session State

```rust
pub struct Session {
    pub session_id: SessionId,
    pub user_id: String,
    pub created_at: SystemTime,
    pub last_active: SystemTime,
    pub state: SessionState,
    pub settings: SessionSettings,
    pub transaction_id: Option<TransactionId>,
    pub cursors: HashMap<String, Cursor>,
    pub prepared_statements: HashMap<String, PreparedStatement>,
    pub session_variables: HashMap<String, String>,
}

pub enum SessionState {
    Active,
    Idle,
    Suspended,
    Killed,
}
```

### Session Configuration

```rust
pub struct SessionConfig {
    pub idle_timeout: Duration,
    pub max_session_lifetime: Duration,
    pub enable_resource_limits: bool,
    pub enable_pooling: bool,
    pub enable_triggers: bool,
}
```

### Authentication

#### Authentication Providers

```rust
pub trait AuthenticationProvider: Send + Sync {
    async fn authenticate(&self, credentials: Credentials) -> Result<AuthenticationResult>;
    async fn validate_session(&self, session_id: SessionId) -> Result<bool>;
    async fn revoke_session(&self, session_id: SessionId) -> Result<()>;
}
```

**Supported Methods:**

- **Database**: Native database authentication
- **LDAP**: Active Directory / LDAP
- **Kerberos**: Kerberos/GSSAPI
- **SAML**: SAML 2.0
- **OAuth/OIDC**: OAuth 2.0 / OpenID Connect
- **Token**: JWT token-based
- **Certificate**: X.509 client certificates

#### Example Authentication

```rust
use rusty_db::pool::{SessionManager, Credentials};

let manager = SessionManager::new(config).await?;

// Authenticate with credentials
let credentials = Credentials::UsernamePassword {
    username: "alice".to_string(),
    password: "secret".to_string(),
};

let session = manager.create_session(credentials).await?;
println!("Session ID: {:?}", session.session_id);
```

### Resource Control

```rust
pub struct ResourceController {
    limits: Arc<RwLock<HashMap<String, ResourceLimits>>>,
    usage: Arc<RwLock<HashMap<SessionId, ResourceUsage>>>,
}

pub struct ResourceLimits {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_seconds: Option<u64>,
    pub max_io_bytes: Option<u64>,
    pub max_open_cursors: Option<usize>,
    pub max_sessions_per_user: Option<usize>,
    pub max_parallel_queries: Option<usize>,
}
```

**Example:**

```rust
let limits = ResourceLimits {
    max_memory_mb: Some(512),
    max_cpu_seconds: Some(300),
    max_io_bytes: Some(1_000_000_000),
    max_open_cursors: Some(100),
    max_sessions_per_user: Some(10),
    max_parallel_queries: Some(4),
};

manager.set_user_limits("analyst_role", limits).await?;
```

### Session Pooling (DRCP-like)

```rust
pub struct SessionPool {
    config: PoolConfig,
    sessions: Arc<RwLock<HashMap<String, Vec<Session>>>>,
    purity_levels: HashMap<String, PurityLevel>,
}

pub enum PurityLevel {
    New,        // Fresh session required
    Self_,      // Session reuse by same user
    Any,        // Any session can be reused
}
```

**Benefits:**

- Reduced session creation overhead
- Session multiplexing
- Tag-based session selection
- Automatic session cleanup

**Example:**

```rust
// Acquire pooled session with tag
let session = pool.acquire_session(
    "web_app",
    Some("connection_class=webapp,language=en"),
    PurityLevel::Self_
).await?;
```

### Session Events

```rust
pub struct SessionEventManager {
    triggers: HashMap<SessionTrigger, Vec<SessionCallback>>,
}

pub enum SessionTrigger {
    Login,
    Logoff,
    Suspend,
    Resume,
    Timeout,
    Kill,
}

pub trait SessionCallback: Send + Sync {
    async fn on_event(&self, event: &SessionEvent) -> Result<()>;
}
```

**Example:**

```rust
// Register login trigger
manager.register_trigger(
    SessionTrigger::Login,
    Box::new(|event| {
        Box::pin(async move {
            println!("User {} logged in", event.user_id);
            Ok(())
        })
    })
).await;
```

### Session Operations

```http
# List Active Sessions
GET /api/v1/pool/sessions

# Get Session Details
GET /api/v1/pool/sessions/{session_id}

# Terminate Session
DELETE /api/v1/pool/sessions/{session_id}

# Kill Connection
DELETE /api/v1/pool/connections/{connection_id}
```

### Session Monitoring

```rust
pub struct SessionStatistics {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub idle_sessions: usize,
    pub pooled_sessions: usize,
    pub avg_session_lifetime_s: f64,
    pub avg_idle_time_s: f64,
    pub sessions_created: u64,
    pub sessions_terminated: u64,
    pub session_timeouts: u64,
}
```

---

## Security & Authentication

Comprehensive security features across all API layers.

### Transport Security

#### TLS/SSL Configuration

```rust
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
    pub ca_cert_path: Option<String>,
    pub min_tls_version: TlsVersion,
    pub cipher_suites: Vec<CipherSuite>,
    pub require_client_cert: bool,
}
```

**Example:**

```rust
let tls_config = TlsConfig {
    cert_path: "/etc/rustydb/server.crt".to_string(),
    key_path: "/etc/rustydb/server.key".to_string(),
    ca_cert_path: Some("/etc/rustydb/ca.crt".to_string()),
    min_tls_version: TlsVersion::TLS1_3,
    cipher_suites: vec![
        CipherSuite::TLS_AES_256_GCM_SHA384,
        CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
    ],
    require_client_cert: true,
};
```

### Authentication Methods Summary

| Method | Use Case | Security | Performance |
|--------|----------|----------|-------------|
| JWT | Web APIs, mobile apps | High | Very High |
| OAuth 2.0 | Third-party integrations | Very High | High |
| API Keys | Service-to-service | Medium | Very High |
| mTLS | High-security environments | Very High | High |
| LDAP | Enterprise SSO | High | Medium |
| Kerberos | Enterprise Windows | Very High | Medium |
| SAML | Enterprise federation | Very High | Medium |

### Authorization Model

#### Role-Based Access Control (RBAC)

```rust
pub struct Role {
    pub name: String,
    pub permissions: Vec<Permission>,
    pub inherits_from: Vec<String>,
}

pub struct Permission {
    pub resource: String,      // e.g., "tables:users"
    pub action: String,        // e.g., "read", "write", "delete"
    pub conditions: Vec<Condition>,
}
```

**Example:**

```rust
let analyst_role = Role {
    name: "analyst".to_string(),
    permissions: vec![
        Permission {
            resource: "tables:*".to_string(),
            action: "read".to_string(),
            conditions: vec![],
        },
        Permission {
            resource: "queries:*".to_string(),
            action: "execute".to_string(),
            conditions: vec![],
        },
    ],
    inherits_from: vec!["user".to_string()],
};
```

#### Attribute-Based Access Control (ABAC)

```rust
pub struct AbacPolicy {
    pub subject: HashMap<String, String>,   // User attributes
    pub resource: HashMap<String, String>,  // Resource attributes
    pub action: String,
    pub environment: HashMap<String, String>,
    pub effect: Effect,
}
```

**Example:**

```rust
let policy = AbacPolicy {
    subject: hashmap!{
        "role" => "analyst",
        "department" => "sales"
    },
    resource: hashmap!{
        "table" => "sales_data",
        "classification" => "internal"
    },
    action: "read".to_string(),
    environment: hashmap!{
        "time" => "business_hours",
        "location" => "office_network"
    },
    effect: Effect::Allow,
};
```

### Request Validation

```rust
pub struct RequestValidator {
    max_request_size: usize,
    allowed_content_types: Vec<String>,
    csrf_protection: bool,
    sql_injection_detection: bool,
    xss_detection: bool,
}
```

### Encryption

#### At Rest

- **Transparent Data Encryption (TDE)**: Full database encryption
- **Column-Level Encryption**: Sensitive field encryption
- **Backup Encryption**: Encrypted backups

#### In Transit

- **TLS 1.3**: Modern TLS protocol
- **mTLS**: Mutual authentication
- **End-to-End Encryption**: Application-level encryption

### Audit Logging

All security events are logged:

```rust
pub struct SecurityEvent {
    pub event_type: SecurityEventType,
    pub request_id: String,
    pub user_id: Option<String>,
    pub client_ip: IpAddr,
    pub resource: String,
    pub action: String,
    pub result: EventResult,
    pub timestamp: SystemTime,
    pub metadata: HashMap<String, String>,
}

pub enum SecurityEventType {
    AuthenticationSuccess,
    AuthenticationFailure,
    AuthorizationSuccess,
    AuthorizationFailure,
    RequestBlocked,
    RateLimitExceeded,
    SuspiciousActivity,
}
```

---

## Monitoring & Observability

### Prometheus Metrics

**Endpoint**: `GET /api/v1/monitoring/prometheus`

```prometheus
# Connection Pool Metrics
rustydb_pool_total_connections{pool="main"} 50
rustydb_pool_idle_connections{pool="main"} 30
rustydb_pool_active_connections{pool="main"} 20
rustydb_pool_wait_queue_size{pool="main"} 5

# API Metrics
rustydb_api_requests_total{method="POST",endpoint="/query",status="200"} 1000
rustydb_api_request_duration_seconds{method="POST",endpoint="/query"} 0.015
rustydb_api_errors_total{type="validation_error"} 10

# Network Metrics
rustydb_network_connections_total 150
rustydb_network_bytes_sent_total 1048576
rustydb_network_bytes_received_total 524288

# Query Metrics
rustydb_query_execution_duration_seconds{type="select"} 0.025
rustydb_query_rows_returned_total{type="select"} 5000

# Transaction Metrics
rustydb_transactions_active 5
rustydb_transactions_committed_total 1000
rustydb_transactions_rolled_back_total 50
```

### Health Checks

#### Liveness Probe

```http
GET /health/live
Response: 200 OK
{ "status": "alive" }
```

**Use**: Kubernetes liveness probe - restart if unhealthy

#### Readiness Probe

```http
GET /health/ready
Response: 200 OK
{
  "status": "ready",
  "checks": {
    "database": "ok",
    "storage": "ok",
    "replication": "ok"
  }
}
```

**Use**: Kubernetes readiness probe - route traffic when ready

#### Startup Probe

```http
GET /health/startup
Response: 200 OK
{ "status": "started" }
```

**Use**: Kubernetes startup probe - delay liveness checks during startup

### Distributed Tracing

Integration with OpenTelemetry:

```rust
pub struct DistributedTracingManager {
    tracer: Tracer,
    propagator: TraceContextPropagator,
    exporter: SpanExporter,
}

pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub baggage: HashMap<String, String>,
}
```

**Trace Headers:**

```http
POST /api/v1/query
traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
tracestate: rustydb=t61rcWkgMzE
```

### Logging

Structured logging with multiple levels:

```rust
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

pub struct LogEntry {
    pub level: LogLevel,
    pub timestamp: SystemTime,
    pub module: String,
    pub message: String,
    pub correlation_id: Option<String>,
    pub metadata: HashMap<String, Value>,
}
```

**Log Formats:**

- JSON (default)
- Logfmt
- Plain text

### Dashboards

Real-time monitoring dashboards:

```http
# Get Dashboard Data
GET /api/v1/monitoring/dashboard

# Dashboard Configuration
GET /api/v1/monitoring/dashboards/{dashboard_id}
```

**Available Dashboards:**

- System Overview
- Connection Pool Status
- Query Performance
- Replication Status
- Security Events
- Resource Usage

---

## Client Integration

### Connection Strings

#### Binary Protocol (Native)

```
rustydb://username:password@host:port/database?options
```

**Example:**

```
rustydb://admin:secret@localhost:5432/mydb?timeout=30&pool_size=10
```

#### PostgreSQL Wire Protocol

```
postgresql://username:password@host:port/database?options
```

**Example:**

```
postgresql://alice:pass@localhost:5432/analytics?sslmode=require
```

### Client Libraries

#### Rust

```rust
use rustydb::client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect(
        "rustydb://admin:secret@localhost:5432/mydb"
    ).await?;

    let rows = client.query("SELECT * FROM users", &[]).await?;

    for row in rows {
        println!("{:?}", row);
    }

    Ok(())
}
```

#### Python

```python
import rustydb

# Connect
client = rustydb.connect(
    "rustydb://admin:secret@localhost:5432/mydb"
)

# Query
rows = client.query("SELECT * FROM users")
for row in rows:
    print(row)

# Transaction
with client.transaction() as txn:
    txn.execute("INSERT INTO users (name) VALUES ('Alice')")
    txn.execute("INSERT INTO users (name) VALUES ('Bob')")
    txn.commit()
```

#### JavaScript/TypeScript

```typescript
import { RustyDBClient } from '@rustydb/client';

const client = new RustyDBClient({
  host: 'localhost',
  port: 5432,
  username: 'admin',
  password: 'secret',
  database: 'mydb',
});

await client.connect();

const result = await client.query('SELECT * FROM users');
console.log(result.rows);

await client.disconnect();
```

#### Java

```java
import io.rustydb.Client;

Client client = new Client.Builder()
    .host("localhost")
    .port(5432)
    .username("admin")
    .password("secret")
    .database("mydb")
    .build();

client.connect();

ResultSet rs = client.query("SELECT * FROM users");
while (rs.next()) {
    System.out.println(rs.getString("name"));
}

client.disconnect();
```

### REST API Client Examples

#### cURL

```bash
# Execute Query
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"sql": "SELECT * FROM users LIMIT 10"}'

# Create Table
curl -X POST http://localhost:8080/api/v1/tables \
  -H "Content-Type: application/json" \
  -d '{
    "name": "products",
    "columns": [
      {"name": "id", "type": "INTEGER", "primary_key": true},
      {"name": "name", "type": "VARCHAR(255)"},
      {"name": "price", "type": "DECIMAL(10,2)"}
    ]
  }'

# Health Check
curl http://localhost:8080/health/ready
```

#### Python (requests)

```python
import requests

# Connect
base_url = "http://localhost:8080/api/v1"
headers = {"Authorization": "Bearer <token>"}

# Query
response = requests.post(
    f"{base_url}/query",
    headers=headers,
    json={"sql": "SELECT * FROM users"}
)
data = response.json()

# Create user
response = requests.post(
    f"{base_url}/admin/users",
    headers=headers,
    json={
        "username": "alice",
        "password": "secret",
        "email": "alice@example.com"
    }
)
```

### GraphQL Client Examples

#### Apollo Client (JavaScript)

```javascript
import { ApolloClient, InMemoryCache, gql } from '@apollo/client';

const client = new ApolloClient({
  uri: 'http://localhost:8080/graphql',
  cache: new InMemoryCache(),
  headers: {
    authorization: 'Bearer <token>',
  },
});

// Query
const { data } = await client.query({
  query: gql`
    query GetUsers {
      queryTable(table: "users", limit: 10) {
        ... on QuerySuccess {
          rows { id data }
        }
      }
    }
  `,
});

// Subscription
client.subscribe({
  query: gql`
    subscription {
      tableChanges(table: "users") {
        changeType
        row { id data }
      }
    }
  `,
}).subscribe({
  next: (data) => console.log(data),
  error: (error) => console.error(error),
});
```

---

## Performance & Scalability

### Performance Characteristics

| Operation | Latency (p50) | Latency (p99) | Throughput |
|-----------|---------------|---------------|------------|
| Binary Protocol Query | < 1ms | < 5ms | 100,000 qps |
| REST API Query | < 2ms | < 10ms | 50,000 qps |
| GraphQL Query | < 3ms | < 15ms | 30,000 qps |
| WebSocket Message | < 1ms | < 3ms | 200,000 msg/s |

### Scalability Limits

- **Maximum Concurrent Connections**: 10,000 (TCP server)
- **Maximum Concurrent WebSockets**: 50,000
- **Maximum Active Queries**: 100,000
- **Maximum Active Sessions**: 50,000
- **Maximum Batch Size**: 1,000 statements
- **Maximum Request Size**: 1 MB (configurable up to 16 MB)

### Optimization Tips

#### Connection Pooling

```rust
// Optimal configuration for high concurrency
let config = PoolConfig::builder()
    .min_size(50)                    // Keep warm connections
    .max_size(500)                   // High ceiling
    .acquire_timeout(Duration::from_secs(5))
    .validate_on_acquire(false)      // Disable for performance
    .statement_cache_size(1000)      // Large cache
    .enable_partitioning(true)       // Isolate workloads
    .build()?;
```

#### Query Performance

- Use prepared statements for repeated queries
- Enable statement caching in connection pool
- Use connection pooling to reduce overhead
- Batch operations when possible
- Use appropriate indexes

#### API Performance

- Enable HTTP/2 for multiplexing
- Use compression (gzip, brotli)
- Enable caching headers
- Use CDN for static content
- Implement GraphQL persisted queries

#### Network Performance

- Enable TCP_NODELAY for low latency
- Use connection pooling
- Enable keep-alive
- Use binary protocol for best performance
- Consider WebSocket for streaming

---

## Configuration Reference

### Environment Variables

```bash
# Network
RUSTYDB_LISTEN_ADDR=0.0.0.0
RUSTYDB_PORT=5432
RUSTYDB_MAX_CONNECTIONS=10000

# API
RUSTYDB_API_PORT=8080
RUSTYDB_API_ENABLE_CORS=true
RUSTYDB_API_RATE_LIMIT=100
RUSTYDB_API_TIMEOUT=30

# Security
RUSTYDB_ENABLE_TLS=true
RUSTYDB_TLS_CERT=/etc/rustydb/server.crt
RUSTYDB_TLS_KEY=/etc/rustydb/server.key
RUSTYDB_REQUIRE_AUTH=true

# Connection Pool
RUSTYDB_POOL_MIN_SIZE=10
RUSTYDB_POOL_MAX_SIZE=100
RUSTYDB_POOL_TIMEOUT=30

# Monitoring
RUSTYDB_ENABLE_METRICS=true
RUSTYDB_METRICS_PORT=9090
RUSTYDB_LOG_LEVEL=info
```

### Configuration File (TOML)

```toml
# /etc/rustydb/config.toml

[network]
listen_addr = "0.0.0.0"
port = 5432
max_connections = 10000
max_request_size = 1048576  # 1 MB

[api]
port = 8080
enable_cors = true
cors_origins = ["http://localhost:3000"]
rate_limit_rps = 100
request_timeout_secs = 30
max_body_size = 10485760  # 10 MB
enable_swagger = true
enable_auth = true

[api.graphql]
max_depth = 10
max_complexity = 1000
enable_introspection = false
enable_playground = false

[pool]
min_size = 10
max_size = 100
initial_size = 20
acquire_timeout_secs = 30
max_lifetime_secs = 3600
idle_timeout_secs = 600
validate_on_acquire = true
statement_cache_size = 100
enable_partitioning = false

[security]
enable_tls = true
tls_cert_path = "/etc/rustydb/server.crt"
tls_key_path = "/etc/rustydb/server.key"
require_client_cert = false
min_tls_version = "1.3"

[monitoring]
enable_metrics = true
metrics_port = 9090
enable_prometheus = true
enable_tracing = false
log_level = "info"
log_format = "json"
```

---

## Known Issues & Limitations

### Current Limitations

1. **PostgreSQL Wire Protocol**: Basic compatibility only, not full PostgreSQL protocol support
2. **Monolithic Router**: REST API router contains 1688 lines and 300+ routes (refactoring recommended)
3. **Buffer Allocation**: Each connection allocates 1MB buffer (10,000 connections = ~10GB RAM)
4. **Rate Limiter Duplication**: 6 different rate limiter implementations (consolidation recommended)
5. **Connection Pool Duplication**: 4 different connection pool implementations (trait abstraction recommended)

### Performance Considerations

- **Memory Usage**: Each connection uses ~1MB for request buffer
- **Connection Limits**: Default limit of 10,000 concurrent connections
- **Request Size**: Maximum request size of 1MB (prevents DoS but limits large queries)
- **Session Tracking**: Unbounded session/query tracking requires manual cleanup

### Security Considerations

- **GraphQL Introspection**: Should be disabled in production
- **GraphQL Playground**: Should be disabled in production
- **CORS Origins**: Configure specific origins, never use "*" wildcard
- **Rate Limiting**: Configure appropriate limits for your use case
- **TLS**: Always use TLS in production environments

---

## Appendix

### Port Allocation

| Service | Default Port | Protocol | Purpose |
|---------|--------------|----------|---------|
| Database Server | 5432 | TCP (Binary) | Native RustyDB protocol |
| REST API | 8080 | HTTP/HTTPS | REST endpoints |
| GraphQL | 8080/graphql | HTTP/HTTPS | GraphQL queries |
| WebSocket | 8080/ws | WS/WSS | Real-time streaming |
| Prometheus | 9090 | HTTP | Metrics export |
| Cluster | 7000-7999 | TCP | Inter-node communication |

### Error Codes

| Code | Description | HTTP Status |
|------|-------------|-------------|
| AUTHENTICATION_FAILED | Authentication failure | 401 |
| AUTHORIZATION_FAILED | Insufficient permissions | 403 |
| RATE_LIMIT_EXCEEDED | Rate limit exceeded | 429 |
| VALIDATION_ERROR | Request validation failed | 400 |
| NOT_FOUND | Resource not found | 404 |
| INTERNAL_ERROR | Internal server error | 500 |
| SERVICE_UNAVAILABLE | Service temporarily unavailable | 503 |
| POOL_EXHAUSTED | Connection pool exhausted | 503 |
| TIMEOUT | Request timeout | 504 |

### Support & Resources

- **Documentation**: https://rustydb.io/docs
- **API Reference**: https://rustydb.io/api
- **GitHub**: https://github.com/rustydb/rustydb
- **Discord**: https://discord.gg/rustydb
- **Stack Overflow**: Tag `rustydb`

---

**End of Document**

*RustyDB v0.5.1 Network & API Layer Documentation*
*Copyright © 2025 RustyDB Contributors. All rights reserved.*
