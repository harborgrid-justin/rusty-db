# RustyDB WebSocket API Reference

**RustyDB v0.6.5 - Enterprise Server ($856M Release)**
**WebSocket Protocol**: RFC 6455
**Last Updated**: 2025-12-29
**Base URL**: `ws://localhost:8080/api/v1/stream`

> **Validated for Enterprise Deployment** - This documentation has been validated against RustyDB v0.6.5 production builds and is certified for enterprise use.

---

## Table of Contents

1. [Overview](#overview)
2. [WebSocket Endpoints](#websocket-endpoints)
3. [Connection Guide](#connection-guide)
4. [Message Format](#message-format)
5. [Query Streaming](#query-streaming)
6. [Real-Time Subscriptions](#real-time-subscriptions)
7. [Metrics Streaming](#metrics-streaming)
8. [Authentication](#authentication)
9. [Error Handling](#error-handling)
10. [Best Practices](#best-practices)
11. [Security Considerations](#security-considerations)

---

## Overview

RustyDB provides WebSocket support for real-time, bidirectional communication between clients and the database server. WebSocket connections enable:

- **Streaming Query Results**: Execute queries and receive results in real-time
- **Live Metrics**: Subscribe to database metrics and performance data
- **Real-time Notifications**: Get notified of database events and changes
- **GraphQL Subscriptions**: Real-time GraphQL subscription support
- **Low Latency**: Persistent connections eliminate HTTP handshake overhead

### Key Features

- Full-duplex communication
- Automatic reconnection with exponential backoff
- Heartbeat/ping-pong for connection health
- Message multiplexing (multiple subscriptions per connection)
- Authentication and authorization
- Rate limiting per connection
- Compression support

### Architecture

```
┌─────────────┐           WebSocket            ┌─────────────┐
│   Client    │ ◄──────────────────────────► │  RustyDB    │
│             │    ws://host:port/path        │   Server    │
└─────────────┘                                └─────────────┘
      │                                              │
      │  1. Connect                                  │
      │ ─────────────────────────────────────────► │
      │                                              │
      │  2. Authenticate                             │
      │ ─────────────────────────────────────────► │
      │                                              │
      │  3. Subscribe                                │
      │ ─────────────────────────────────────────► │
      │                                              │
      │  4. Stream Data                              │
      │ ◄───────────────────────────────────────── │
      │                                              │
      │  5. Heartbeat (ping/pong)                    │
      │ ◄──────────────────────────────────────── ►│
```

---

## WebSocket Endpoints

### 1. Query Streaming Endpoint

**URL**: `ws://localhost:8080/api/v1/stream`

Execute SQL queries and receive results as a stream.

**Use Cases**:
- Large result sets
- Long-running queries
- Progressive result rendering
- Real-time query execution

### 2. GraphQL Subscriptions Endpoint

**URL**: `ws://localhost:8080/graphql/ws`

GraphQL subscriptions over WebSocket using the `graphql-ws` protocol.

**Use Cases**:
- Real-time data updates
- Event notifications
- Live dashboards
- Collaborative applications

### 3. Metrics Streaming Endpoint

**URL**: `ws://localhost:8080/api/v1/metrics/stream`

Subscribe to real-time database metrics and performance data.

**Use Cases**:
- Monitoring dashboards
- Performance analysis
- Alert systems
- Real-time analytics

### 4. General Events Endpoint

**URL**: `ws://localhost:8080/api/v1/ws/events`

Subscribe to database events and changes.

**Use Cases**:
- Change data capture
- Event-driven architectures
- Audit logging
- Replication monitoring

---

## Connection Guide

### Basic Connection (JavaScript)

```javascript
const ws = new WebSocket('ws://localhost:8080/api/v1/stream');

ws.onopen = () => {
  console.log('Connected to RustyDB');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = (event) => {
  console.log('Disconnected:', event.code, event.reason);
};
```

### Connection with Authentication

```javascript
// Option 1: Token in query parameter
const ws = new WebSocket('ws://localhost:8080/api/v1/stream?token=YOUR_JWT_TOKEN');

// Option 2: Token in first message (recommended)
const ws = new WebSocket('ws://localhost:8080/api/v1/stream');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'auth',
    payload: {
      token: 'YOUR_JWT_TOKEN'
    },
    timestamp: new Date().toISOString()
  }));
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  if (message.type === 'auth_success') {
    console.log('Authenticated successfully');
    // Now you can send queries
  } else if (message.type === 'auth_error') {
    console.error('Authentication failed:', message.payload.message);
    ws.close();
  }
};
```

### Connection States

| State | Value | Description |
|-------|-------|-------------|
| `CONNECTING` | 0 | Connection is being established |
| `OPEN` | 1 | Connection is established and ready |
| `CLOSING` | 2 | Connection is being closed |
| `CLOSED` | 3 | Connection is closed |

### URL Schemes

| Scheme | Description | Security |
|--------|-------------|----------|
| `ws://` | Unencrypted WebSocket | Development only |
| `wss://` | Encrypted WebSocket (TLS/SSL) | Production (required) |

**Production Example**:
```javascript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/api/v1/stream`);
```

---

## Message Format

All WebSocket messages use JSON format with a standardized structure.

### Client → Server Messages

```json
{
  "type": "query|subscribe|unsubscribe|ping|auth",
  "payload": {},
  "timestamp": "2025-12-29T10:30:00.000Z",
  "id": "unique-message-id"
}
```

**Fields**:
- `type` (string, required): Message type
- `payload` (object, required): Message-specific data
- `timestamp` (string, required): ISO 8601 timestamp
- `id` (string, optional): Unique message ID for correlation

### Server → Client Messages

```json
{
  "type": "data|error|pong|auth_success|auth_error",
  "payload": {},
  "timestamp": "2025-12-29T10:30:00.001Z",
  "id": "correlation-id"
}
```

**Fields**:
- `type` (string, required): Message type
- `payload` (object, required): Response data or error details
- `timestamp` (string, required): ISO 8601 timestamp
- `id` (string, optional): Correlation ID (matches request ID)

---

## Query Streaming

### Execute Streaming Query

**Request**:
```json
{
  "type": "query",
  "payload": {
    "sql": "SELECT * FROM users WHERE age > 18 ORDER BY name",
    "params": [],
    "streaming": true
  },
  "timestamp": "2025-12-29T10:30:00.000Z",
  "id": "query-001"
}
```

**Response (Success)**:
```json
{
  "type": "data",
  "payload": {
    "status": "success",
    "rows": [
      [1, "Alice", "alice@example.com", 25],
      [2, "Bob", "bob@example.com", 30]
    ],
    "columns": ["id", "name", "email", "age"],
    "rows_affected": 0,
    "has_more": false,
    "execution_time_ms": 45
  },
  "timestamp": "2025-12-29T10:30:00.050Z",
  "id": "query-001"
}
```

**Response (Error)**:
```json
{
  "type": "error",
  "payload": {
    "status": "error",
    "message": "Table 'users' not found",
    "code": "TABLE_NOT_FOUND"
  },
  "timestamp": "2025-12-29T10:30:00.050Z",
  "id": "query-001"
}
```

### Streaming Large Result Sets

For large result sets, the server streams rows in chunks:

**Initial Response**:
```json
{
  "type": "data",
  "payload": {
    "status": "streaming",
    "columns": ["id", "name", "email", "age"],
    "total_rows": 100000,
    "chunk_size": 1000,
    "chunk_number": 1
  },
  "id": "query-001"
}
```

**Chunk 1**:
```json
{
  "type": "data",
  "payload": {
    "status": "chunk",
    "rows": [[1, "Alice", "alice@example.com", 25], ...],
    "chunk_number": 1,
    "has_more": true
  },
  "id": "query-001"
}
```

**Final Chunk**:
```json
{
  "type": "data",
  "payload": {
    "status": "complete",
    "rows": [[100000, "Zoe", "zoe@example.com", 42]],
    "chunk_number": 100,
    "has_more": false,
    "total_time_ms": 5678
  },
  "id": "query-001"
}
```

---

## Real-Time Subscriptions

### Subscribe to Table Changes

**Request**:
```json
{
  "type": "subscribe",
  "payload": {
    "subscription_type": "table_changes",
    "table": "users",
    "events": ["INSERT", "UPDATE", "DELETE"],
    "filter": {
      "active": true
    }
  },
  "id": "sub-001"
}
```

**Response (Subscription Confirmed)**:
```json
{
  "type": "subscribed",
  "payload": {
    "subscription_id": "sub-001",
    "status": "active"
  },
  "id": "sub-001"
}
```

**Event Notification**:
```json
{
  "type": "event",
  "payload": {
    "subscription_id": "sub-001",
    "event_type": "INSERT",
    "table": "users",
    "data": {
      "id": 124,
      "name": "New User",
      "email": "new@example.com",
      "active": true
    },
    "timestamp": "2025-12-29T10:35:00.000Z"
  }
}
```

### Unsubscribe

**Request**:
```json
{
  "type": "unsubscribe",
  "payload": {
    "subscription_id": "sub-001"
  }
}
```

**Response**:
```json
{
  "type": "unsubscribed",
  "payload": {
    "subscription_id": "sub-001",
    "status": "closed"
  }
}
```

---

## Metrics Streaming

### Subscribe to Metrics

**Request**:
```json
{
  "type": "subscribe",
  "payload": {
    "subscription_type": "metrics",
    "metrics": ["cpu", "memory", "connections", "queries_per_second"],
    "interval_seconds": 5
  },
  "id": "metrics-001"
}
```

**Metrics Update**:
```json
{
  "type": "metrics",
  "payload": {
    "subscription_id": "metrics-001",
    "metrics": {
      "cpu": 45.2,
      "memory": 1024000000,
      "connections": 145,
      "queries_per_second": 1234.56
    },
    "timestamp": "2025-12-29T10:30:00.000Z"
  }
}
```

---

## Authentication

### JWT Token Authentication

WebSocket connections support JWT token authentication similar to REST API.

#### Method 1: Query Parameter

```javascript
const token = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...';
const ws = new WebSocket(`ws://localhost:8080/api/v1/stream?token=${token}`);
```

**Pros**: Simple, works with any WebSocket client
**Cons**: Token visible in logs, not recommended for production

#### Method 2: Authentication Message (Recommended)

```javascript
const ws = new WebSocket('ws://localhost:8080/api/v1/stream');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'auth',
    payload: {
      token: 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...'
    },
    timestamp: new Date().toISOString()
  }));
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  if (message.type === 'auth_success') {
    console.log('Authenticated successfully');
    // Now you can send queries
  } else if (message.type === 'auth_error') {
    console.error('Authentication failed:', message.payload.message);
    ws.close();
  }
};
```

**Pros**: Token not in URL, more secure
**Cons**: Requires initial handshake

### Obtaining JWT Token

Use the REST API login endpoint:

```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'
```

**Response**:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_at": "2025-12-29T11:00:00.000Z"
  }
}
```

---

## Error Handling

### Error Message Format

```json
{
  "type": "error",
  "payload": {
    "status": "error",
    "message": "Human-readable error message",
    "code": "ERROR_CODE",
    "details": {}
  },
  "timestamp": "2025-12-29T10:30:00.000Z",
  "id": "correlation-id"
}
```

### Common Error Codes

| Code | Description | Action |
|------|-------------|--------|
| `AUTH_REQUIRED` | Authentication required | Send auth message |
| `AUTH_FAILED` | Invalid credentials | Check token validity |
| `AUTH_EXPIRED` | Token expired | Refresh token and reconnect |
| `PERMISSION_DENIED` | Insufficient permissions | Check user privileges |
| `INVALID_MESSAGE` | Malformed message | Check message format |
| `QUERY_ERROR` | SQL query error | Fix SQL syntax |
| `TABLE_NOT_FOUND` | Table doesn't exist | Verify table name |
| `CONNECTION_LIMIT` | Too many connections | Retry later |
| `RATE_LIMIT_EXCEEDED` | Too many requests | Slow down request rate |
| `INTERNAL_ERROR` | Server error | Contact administrator |

### Connection Closure Codes

Following [RFC 6455](https://datatracker.ietf.org/doc/html/rfc6455#section-7.4):

| Code | Name | Description |
|------|------|-------------|
| 1000 | Normal Closure | Normal connection closure |
| 1001 | Going Away | Server shutdown or client navigation |
| 1002 | Protocol Error | Protocol violation |
| 1003 | Unsupported Data | Unsupported message type |
| 1007 | Invalid Frame Payload Data | Invalid UTF-8 or message format |
| 1008 | Policy Violation | Authentication failure |
| 1009 | Message Too Big | Message exceeds size limit |
| 1011 | Internal Server Error | Unexpected server error |

### Error Handling Example

```javascript
ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = (event) => {
  console.log('Connection closed:', event.code, event.reason);

  switch (event.code) {
    case 1000:
      console.log('Normal closure');
      break;
    case 1008:
      console.log('Authentication failed, please re-login');
      // Redirect to login page
      break;
    case 1011:
      console.log('Server error, retrying in 5 seconds...');
      setTimeout(() => reconnect(), 5000);
      break;
    default:
      console.log('Unexpected closure, reconnecting...');
      reconnect();
  }
};
```

---

## Best Practices

### 1. Implement Reconnection Logic

```javascript
class RustyDBWebSocket {
  constructor(url) {
    this.url = url;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    this.reconnectDelay = 3000; // 3 seconds
    this.connect();
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('Connected');
      this.reconnectAttempts = 0;
    };

    this.ws.onclose = (event) => {
      if (event.code !== 1000 && this.reconnectAttempts < this.maxReconnectAttempts) {
        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
        console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
        setTimeout(() => this.connect(), delay);
      }
    };
  }
}
```

### 2. Heartbeat Implementation

```javascript
let heartbeatInterval;

ws.onopen = () => {
  // Send ping every 30 seconds
  heartbeatInterval = setInterval(() => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'ping',
        timestamp: new Date().toISOString()
      }));
    }
  }, 30000);
};

ws.onclose = () => {
  clearInterval(heartbeatInterval);
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  if (message.type === 'pong') {
    console.log('Heartbeat acknowledged');
  }
};
```

### 3. Message Queue for Offline Messages

```javascript
class MessageQueue {
  constructor(ws) {
    this.ws = ws;
    this.queue = [];
  }

  send(message) {
    if (this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
      this.flushQueue();
    } else {
      this.queue.push(message);
    }
  }

  flushQueue() {
    while (this.queue.length > 0 && this.ws.readyState === WebSocket.OPEN) {
      const message = this.queue.shift();
      this.ws.send(JSON.stringify(message));
    }
  }
}
```

### 4. Limit Concurrent Subscriptions

```javascript
const MAX_CONCURRENT_SUBSCRIPTIONS = 10;
const activeSubscriptions = new Set();

function subscribe(subscription) {
  if (activeSubscriptions.size >= MAX_CONCURRENT_SUBSCRIPTIONS) {
    console.warn('Too many active subscriptions');
    return;
  }

  const id = crypto.randomUUID();
  activeSubscriptions.add(id);

  ws.send(JSON.stringify({
    type: 'subscribe',
    payload: subscription,
    id
  }));

  return () => {
    activeSubscriptions.delete(id);
    ws.send(JSON.stringify({ type: 'unsubscribe', id }));
  };
}
```

---

## Security Considerations

### 1. Always Use WSS in Production

```javascript
// Production: Use encrypted connection
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/api/v1/stream`);
```

**Configuration Requirements**:
- Enable TLS 1.2 or higher
- Use strong cipher suites
- Implement certificate validation
- Enable HSTS (HTTP Strict Transport Security)

### 2. Token Management

```javascript
// Store tokens securely
const token = localStorage.getItem('rustydb_token');

// Validate token expiration
function isTokenExpired(token) {
  const payload = JSON.parse(atob(token.split('.')[1]));
  return Date.now() >= payload.exp * 1000;
}

// Refresh expired tokens
if (isTokenExpired(token)) {
  const newToken = await refreshToken();
  localStorage.setItem('rustydb_token', newToken);
}
```

### 3. Input Validation

```javascript
// Bad: SQL injection risk
const sql = `SELECT * FROM users WHERE name = '${userInput}'`;

// Good: Use parameterized queries
ws.send(JSON.stringify({
  type: 'query',
  payload: {
    sql: 'SELECT * FROM users WHERE name = ?',
    params: [userInput]
  }
}));
```

### 4. Rate Limiting

Server implements per-connection rate limiting:
- Default: 100 messages per second
- Exceeding limit results in temporary throttling
- Persistent violations lead to connection closure

Client-side rate limiting:

```javascript
class RateLimiter {
  constructor(maxPerSecond) {
    this.max = maxPerSecond;
    this.count = 0;
    this.resetTime = Date.now() + 1000;
  }

  async acquire() {
    if (Date.now() >= this.resetTime) {
      this.count = 0;
      this.resetTime = Date.now() + 1000;
    }

    if (this.count >= this.max) {
      await new Promise(resolve =>
        setTimeout(resolve, this.resetTime - Date.now())
      );
      return this.acquire();
    }

    this.count++;
  }
}
```

---

## Configuration Options

### Server-Side Configuration

```rust
use rusty_db::api::ApiConfig;

let config = ApiConfig {
    // WebSocket settings
    websocket_max_connections: 1000,
    websocket_max_message_size: 1024 * 1024, // 1 MB
    websocket_ping_interval: 30, // seconds
    websocket_timeout: 60, // seconds

    // Security settings
    require_authentication: true,
    enable_rate_limiting: true,
    rate_limit_rps: 100,

    // TLS settings
    enable_tls: true,
    tls_cert_path: "/path/to/cert.pem",
    tls_key_path: "/path/to/key.pem",

    ..Default::default()
};
```

### Client-Side Configuration

```javascript
const config = {
  url: 'wss://localhost:8080/api/v1/stream',

  // Reconnection
  reconnect: true,
  maxReconnectAttempts: 5,
  reconnectDelay: 3000,

  // Heartbeat
  heartbeat: true,
  heartbeatInterval: 30000,

  // Timeouts
  connectionTimeout: 10000,
  requestTimeout: 30000,

  // Authentication
  token: null,
  autoAuth: true,

  // Debugging
  debug: false,
  logLevel: 'info'
};
```

---

## Complete Client Example

### JavaScript/TypeScript

```javascript
class RustyDBClient {
  constructor(url, token) {
    this.url = url;
    this.token = token;
    this.ws = null;
    this.subscriptions = new Map();
    this.messageHandlers = new Map();
    this.connect();
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('Connected to RustyDB');
      this.authenticate();
    };

    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      this.handleMessage(message);
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    this.ws.onclose = (event) => {
      console.log('Disconnected:', event.code, event.reason);
      this.reconnect();
    };
  }

  authenticate() {
    this.send({
      type: 'auth',
      payload: { token: this.token }
    });
  }

  send(message) {
    if (this.ws.readyState === WebSocket.OPEN) {
      message.timestamp = new Date().toISOString();
      message.id = message.id || crypto.randomUUID();
      this.ws.send(JSON.stringify(message));
      return message.id;
    } else {
      throw new Error('WebSocket not connected');
    }
  }

  query(sql, params = []) {
    return new Promise((resolve, reject) => {
      const id = this.send({
        type: 'query',
        payload: { sql, params, streaming: true }
      });

      this.messageHandlers.set(id, (message) => {
        if (message.type === 'data') {
          resolve(message.payload);
        } else if (message.type === 'error') {
          reject(new Error(message.payload.message));
        }
        this.messageHandlers.delete(id);
      });
    });
  }

  subscribe(subscription) {
    const id = this.send({
      type: 'subscribe',
      payload: subscription
    });

    this.subscriptions.set(id, subscription);
    return id;
  }

  unsubscribe(id) {
    this.send({
      type: 'unsubscribe',
      payload: { subscription_id: id }
    });
    this.subscriptions.delete(id);
  }

  handleMessage(message) {
    const handler = this.messageHandlers.get(message.id);
    if (handler) {
      handler(message);
    } else if (message.type === 'event') {
      // Handle subscription events
      console.log('Event:', message.payload);
    }
  }

  reconnect() {
    setTimeout(() => this.connect(), 3000);
  }

  close() {
    this.ws.close(1000, 'Client closed');
  }
}

// Usage
const client = new RustyDBClient('ws://localhost:8080/api/v1/stream', 'YOUR_TOKEN');

// Execute query
const result = await client.query('SELECT * FROM users WHERE age > ?', [18]);
console.log('Query result:', result);

// Subscribe to table changes
const subscriptionId = client.subscribe({
  subscription_type: 'table_changes',
  table: 'users',
  events: ['INSERT', 'UPDATE', 'DELETE']
});

// Unsubscribe later
client.unsubscribe(subscriptionId);
```

---

## Additional Resources

- [WebSocket RFC 6455](https://datatracker.ietf.org/doc/html/rfc6455)
- [GraphQL over WebSocket Protocol](https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md)
- [MDN WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
- [REST API Reference](./REST_API.md)
- [GraphQL API Reference](./GRAPHQL_API.md)
- [WebSocket Integration Guide](/home/user/rusty-db/docs/WEBSOCKET_INTEGRATION.md)

---

**Validated for Enterprise Deployment** - RustyDB v0.6.5 ($856M Release)

*Last Updated: 2025-12-29*
*Documentation Version: 1.0.0*
