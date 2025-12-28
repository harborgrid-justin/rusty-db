# RustyDB WebSocket API Guide

**RustyDB v0.6.0 - Enterprise Server**
**WebSocket Protocol**: RFC 6455
**Last Updated**: 2025-12-28
**Status**: Production Ready

---

## Table of Contents

1. [Introduction](#introduction)
2. [WebSocket Endpoints](#websocket-endpoints)
3. [Connection Guide](#connection-guide)
4. [Message Format](#message-format)
5. [Query Streaming](#query-streaming)
6. [GraphQL Subscriptions](#graphql-subscriptions)
7. [Authentication](#authentication)
8. [Error Handling](#error-handling)
9. [Best Practices](#best-practices)
10. [Security](#security)

---

## Introduction

RustyDB provides WebSocket support for real-time, bidirectional communication between clients and the database server.

### Key Features

- **Full-Duplex Communication**: Persistent, bidirectional connections
- **Streaming Query Results**: Execute queries and receive results in real-time
- **Live Metrics**: Subscribe to database metrics and performance data
- **Real-time Notifications**: Get notified of database events and changes
- **GraphQL Subscriptions**: Real-time GraphQL subscription support
- **Low Latency**: Eliminates HTTP handshake overhead

### Benefits

| Feature | Description |
|---------|-------------|
| **Lower Latency** | Persistent connections reduce overhead |
| **Streaming Results** | Process large datasets progressively |
| **Real-Time Updates** | Instant notifications of data changes |
| **Reduced Server Load** | Fewer connections, less polling |
| **Efficient Protocol** | Binary frames, message compression |

### Architecture

```
┌─────────────┐          WebSocket           ┌─────────────┐
│   Client    │ ◄──────────────────────────► │  RustyDB    │
│             │   ws://host:port/path        │   Server    │
└─────────────┘                               └─────────────┘
      │                                             │
      │  1. Connect                                 │
      │ ─────────────────────────────────────────►│
      │  2. Authenticate                            │
      │ ─────────────────────────────────────────►│
      │  3. Subscribe                               │
      │ ─────────────────────────────────────────►│
      │  4. Stream Data                             │
      │ ◄────────────────────────────────────────│
      │  5. Heartbeat (ping/pong)                   │
      │ ◄──────────────────────────────────────► │
```

---

## WebSocket Endpoints

### 1. Query Streaming

**URL**: `ws://host:port/api/v1/stream`

Execute SQL queries and receive results as a stream.

**Use Cases**:
- Large result sets
- Long-running queries
- Progressive result rendering
- Real-time query execution

**Status**: Production Ready (v1.0.0)

### 2. GraphQL Subscriptions

**URL**: `ws://host:port/graphql/ws`

GraphQL subscriptions over WebSocket using the `graphql-ws` protocol.

**Use Cases**:
- Real-time data updates
- Event notifications
- Live dashboards
- Collaborative applications

**Status**: Production Ready (v1.0.0)

### 3. Metrics Streaming (Planned)

**URL**: `ws://host:port/api/v1/metrics/stream`

Subscribe to real-time database metrics and performance data.

**Status**: Planned (v0.7.0)

---

## Connection Guide

### Basic Connection (JavaScript)

```javascript
// Establish WebSocket connection
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
// Method 1: Token in query parameter (less secure)
const ws = new WebSocket('ws://localhost:8080/api/v1/stream?token=YOUR_JWT_TOKEN');

// Method 2: Token in first message (recommended)
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
```

### Connection States

| State | Value | Description |
|-------|-------|-------------|
| `CONNECTING` | 0 | Connection is being established |
| `OPEN` | 1 | Connection is established and ready |
| `CLOSING` | 2 | Connection is being closed |
| `CLOSED` | 3 | Connection is closed |

### URL Schemes

| Scheme | Security | Usage |
|--------|----------|-------|
| `ws://` | Unencrypted | Development only |
| `wss://` | TLS/SSL encrypted | Production (required) |

---

## Message Format

All WebSocket messages use JSON format with a standardized structure.

### Client → Server Messages

```json
{
  "type": "query|subscribe|unsubscribe|ping|auth",
  "payload": {},
  "timestamp": "2025-12-28T10:30:00.000Z",
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
  "timestamp": "2025-12-28T10:30:00.001Z",
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
    "sql": "SELECT * FROM users WHERE age > ? ORDER BY name",
    "params": [18],
    "streaming": true
  },
  "timestamp": "2025-12-28T10:30:00.000Z",
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
      [1, "Alice", 25],
      [2, "Bob", 30]
    ],
    "columns": ["id", "name", "age"],
    "rows_affected": 2,
    "has_more": false
  },
  "timestamp": "2025-12-28T10:30:00.050Z",
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
  "timestamp": "2025-12-28T10:30:00.050Z",
  "id": "query-001"
}
```

### Large Result Sets

For large result sets, the server sends multiple messages:

```json
// First chunk
{
  "type": "data",
  "payload": {
    "status": "partial",
    "rows": [ /* First 1000 rows */ ],
    "has_more": true
  },
  "id": "query-001"
}

// Second chunk
{
  "type": "data",
  "payload": {
    "status": "partial",
    "rows": [ /* Next 1000 rows */ ],
    "has_more": true
  },
  "id": "query-001"
}

// Final chunk
{
  "type": "data",
  "payload": {
    "status": "complete",
    "rows": [ /* Last rows */ ],
    "has_more": false,
    "total_rows": 2500
  },
  "id": "query-001"
}
```

---

## GraphQL Subscriptions

### Protocol

RustyDB follows the [graphql-ws protocol](https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md).

### Connection Initialization

```json
{
  "type": "connection_init",
  "payload": {
    "authorization": "Bearer YOUR_JWT_TOKEN"
  }
}
```

**Server Response**:
```json
{
  "type": "connection_ack"
}
```

### Subscribe to Updates

```json
{
  "id": "sub-001",
  "type": "subscribe",
  "payload": {
    "query": "subscription { tableChanges(table: \"users\") { operation row { id fields } } }"
  }
}
```

### Receiving Updates

```json
{
  "id": "sub-001",
  "type": "next",
  "payload": {
    "data": {
      "tableChanges": {
        "operation": "INSERT",
        "row": {
          "id": "123",
          "fields": {
            "name": "Alice",
            "email": "alice@example.com"
          }
        }
      }
    }
  }
}
```

### Unsubscribe

```json
{
  "id": "sub-001",
  "type": "complete"
}
```

---

## Authentication

### JWT Token Authentication

WebSocket connections support JWT token authentication.

#### Method 1: Query Parameter (Simple)

```javascript
const ws = new WebSocket(
  'ws://localhost:8080/api/v1/stream?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...'
);
```

**Pros**: Simple, works with any WebSocket client
**Cons**: Token visible in logs, not recommended for production

#### Method 2: Authentication Message (Recommended)

```javascript
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
  -d '{"username": "admin", "password": "secret123"}'
```

**Response**:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expiresAt": "2025-12-28T11:30:00.000Z"
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
  "timestamp": "2025-12-28T10:30:00.000Z",
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

### Example Error Handling

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

### Connection Management

#### 1. Implement Reconnection Logic

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

#### 2. Heartbeat Implementation

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
```

#### 3. Message Queue for Offline Messages

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

### Performance Optimization

#### 1. Enable Compression

```javascript
// Browser automatically handles compression if server supports it
// For Node.js, use ws library with perMessageDeflate option
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:8080/api/v1/stream', {
  perMessageDeflate: true
});
```

#### 2. Batch Multiple Queries

```javascript
// Good: Single batch
ws.send(JSON.stringify({
  type: 'batch',
  payload: {
    queries: [
      { sql: 'SELECT 1' },
      { sql: 'SELECT 2' }
    ]
  }
}));
```

#### 3. Limit Concurrent Subscriptions

```javascript
const MAX_CONCURRENT_SUBSCRIPTIONS = 10;
const activeSubscriptions = new Set();

function subscribe(query) {
  if (activeSubscriptions.size >= MAX_CONCURRENT_SUBSCRIPTIONS) {
    console.warn('Too many active subscriptions');
    return;
  }

  const id = crypto.randomUUID();
  activeSubscriptions.add(id);

  ws.send(JSON.stringify({
    type: 'subscribe',
    payload: { query },
    id
  }));

  return () => {
    activeSubscriptions.delete(id);
    ws.send(JSON.stringify({ type: 'unsubscribe', id }));
  };
}
```

---

## Security

### Transport Layer Security

#### Always Use WSS in Production

```javascript
// Production: Use encrypted connection
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/api/v1/stream`);
```

**Requirements**:
- TLS 1.2 or higher
- Strong cipher suites
- Certificate validation
- HSTS enabled

### Input Validation

#### Sanitize SQL Queries

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

### Rate Limiting

Server implements per-connection rate limiting:
- Default: 100 messages per second
- Exceeding limit results in temporary throttling
- Persistent violations lead to connection closure

### Data Privacy

- Never log sensitive data in messages
- Implement data masking for PII
- Use Virtual Private Database (VPD) policies
- Enable audit logging for compliance

---

## Code Examples

### Complete Client Implementation (JavaScript)

```javascript
class RustyDBClient {
  constructor(url, token) {
    this.url = url;
    this.token = token;
    this.ws = null;
    this.messageHandlers = new Map();
    this.connect();
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      // Authenticate
      this.send('auth', { token: this.token });
    };

    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      const handlers = this.messageHandlers.get(message.type) || [];
      handlers.forEach(handler => handler(message.payload));
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    this.ws.onclose = (event) => {
      console.log('Connection closed:', event.code);
      if (event.code !== 1000) {
        setTimeout(() => this.connect(), 3000);
      }
    };
  }

  send(type, payload) {
    this.ws.send(JSON.stringify({
      type,
      payload,
      timestamp: new Date().toISOString(),
      id: crypto.randomUUID()
    }));
  }

  on(type, handler) {
    if (!this.messageHandlers.has(type)) {
      this.messageHandlers.set(type, []);
    }
    this.messageHandlers.get(type).push(handler);
  }

  query(sql, params = []) {
    return new Promise((resolve, reject) => {
      const id = crypto.randomUUID();

      const handler = (payload) => {
        if (payload.status === 'success') {
          resolve(payload);
        } else {
          reject(new Error(payload.message));
        }
      };

      this.on('data', handler);
      this.send('query', { sql, params, id });
    });
  }
}

// Usage
const client = new RustyDBClient('ws://localhost:8080/api/v1/stream', 'YOUR_JWT_TOKEN');

client.on('auth_success', () => {
  console.log('Authenticated');

  client.query('SELECT * FROM users WHERE age > ?', [18])
    .then(result => console.log('Query result:', result))
    .catch(error => console.error('Query error:', error));
});
```

---

## Troubleshooting

### Connection Fails

**Symptoms**: Cannot establish WebSocket connection

**Possible Causes**:
1. Server not running → Check `cargo run --bin rusty-db-server`
2. Firewall blocking → Check port 8080 is open
3. TLS certificate issues → Verify certificate validity
4. Proxy/Load balancer → Ensure WebSocket support enabled

### Authentication Errors

**Symptoms**: Connection closes with code 1008

**Possible Causes**:
1. Invalid token → Refresh JWT token
2. Expired token → Re-login
3. Missing permissions → Check user roles

### Message Not Received

**Symptoms**: Messages sent but no response

**Possible Causes**:
1. Wrong message format → Validate JSON structure
2. Server error → Check server logs
3. Connection closed → Verify `readyState === OPEN`

### High Latency

**Symptoms**: Slow message delivery

**Possible Causes**:
1. Network congestion → Check network quality
2. Server overload → Scale horizontally
3. Large messages → Enable compression
4. Too many subscriptions → Reduce concurrent subscriptions

---

## Additional Resources

- **WebSocket RFC 6455**: https://datatracker.ietf.org/doc/html/rfc6455
- **GraphQL over WebSocket Protocol**: https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md
- **MDN WebSocket API**: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket
- **API Overview**: [API_OVERVIEW.md](./API_OVERVIEW.md)
- **GraphQL API**: [GRAPHQL_API.md](./GRAPHQL_API.md)

---

**Last Updated**: 2025-12-28
**WebSocket Protocol**: RFC 6455
**Product Version**: RustyDB v0.6.0 Enterprise Server
