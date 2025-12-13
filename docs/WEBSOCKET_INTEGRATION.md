# WebSocket Integration Guide

**Version**: 1.0.0
**Last Updated**: 2025-12-13
**Status**: Production Ready

## Table of Contents

1. [Overview](#overview)
2. [WebSocket Endpoints](#websocket-endpoints)
3. [Connection Guide](#connection-guide)
4. [Message Format](#message-format)
5. [Subscription Types](#subscription-types)
6. [Authentication](#authentication)
7. [Error Handling](#error-handling)
8. [Best Practices](#best-practices)
9. [Security Considerations](#security-considerations)
10. [Configuration Options](#configuration-options)
11. [Code Examples](#code-examples)

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

**URL**: `ws://host:port/api/v1/stream`

Execute SQL queries and receive results as a stream.

**Use Cases**:
- Large result sets
- Long-running queries
- Progressive result rendering
- Real-time query execution

### 2. GraphQL Subscriptions Endpoint

**URL**: `ws://host:port/graphql/ws`

GraphQL subscriptions over WebSocket using the `graphql-ws` protocol.

**Use Cases**:
- Real-time data updates
- Event notifications
- Live dashboards
- Collaborative applications

### 3. Metrics Streaming (Future)

**URL**: `ws://host:port/api/v1/metrics/stream` (planned)

Subscribe to real-time database metrics and performance data.

**Use Cases**:
- Monitoring dashboards
- Performance analysis
- Alert systems

### 4. Document Store Watch (Future)

**URL**: `ws://host:port/api/v1/documents/collections/{name}/watch` (planned)

MongoDB-like change streams for document collections.

---

## Connection Guide

### Basic Connection

```javascript
// JavaScript/TypeScript
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
// Option 1: Token in query parameter (less secure)
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
```

### Connection States

| State | Description |
|-------|-------------|
| `CONNECTING` (0) | Connection is being established |
| `OPEN` (1) | Connection is established and ready |
| `CLOSING` (2) | Connection is being closed |
| `CLOSED` (3) | Connection is closed |

### URL Schemes

| Scheme | Description | Security |
|--------|-------------|----------|
| `ws://` | Unencrypted WebSocket | Development only |
| `wss://` | Encrypted WebSocket (TLS/SSL) | Production (required) |

---

## Message Format

All WebSocket messages use JSON format with a standardized structure.

### Client → Server Messages

```json
{
  "type": "query|subscribe|unsubscribe|ping|auth",
  "payload": {},
  "timestamp": "2025-12-13T10:30:00.000Z",
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
  "timestamp": "2025-12-13T10:30:00.001Z",
  "id": "correlation-id"
}
```

**Fields**:
- `type` (string, required): Message type
- `payload` (object, required): Response data or error details
- `timestamp` (string, required): ISO 8601 timestamp
- `id` (string, optional): Correlation ID (matches request ID)

---

## Subscription Types

### 1. Query Execution

Execute a SQL query and stream results.

**Request**:
```json
{
  "type": "query",
  "payload": {
    "sql": "SELECT * FROM users WHERE age > 18 ORDER BY name",
    "params": [],
    "streaming": true
  },
  "timestamp": "2025-12-13T10:30:00.000Z",
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
      ["Alice", 25],
      ["Bob", 30]
    ],
    "columns": ["name", "age"],
    "rows_affected": 2,
    "has_more": false
  },
  "timestamp": "2025-12-13T10:30:00.050Z",
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
  "timestamp": "2025-12-13T10:30:00.050Z",
  "id": "query-001"
}
```

### 2. GraphQL Subscriptions

Follow the [graphql-ws protocol](https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md).

**Connection Initialization**:
```json
{
  "type": "connection_init",
  "payload": {
    "authorization": "Bearer YOUR_JWT_TOKEN"
  }
}
```

**Subscription Request**:
```json
{
  "id": "sub-001",
  "type": "subscribe",
  "payload": {
    "query": "subscription { metricUpdated { cpu memory } }"
  }
}
```

**Subscription Data**:
```json
{
  "id": "sub-001",
  "type": "next",
  "payload": {
    "data": {
      "metricUpdated": {
        "cpu": 45.2,
        "memory": 1024000000
      }
    }
  }
}
```

### 3. Heartbeat (Ping/Pong)

Keep connection alive and verify connectivity.

**Client Ping**:
```json
{
  "type": "ping",
  "timestamp": "2025-12-13T10:30:00.000Z"
}
```

**Server Pong**:
```json
{
  "type": "pong",
  "timestamp": "2025-12-13T10:30:00.001Z"
}
```

**Recommended Interval**: 30 seconds

---

## Authentication

### JWT Token Authentication

WebSocket connections support JWT token authentication similar to REST API.

#### Method 1: Query Parameter (Simple)

```
ws://localhost:8080/api/v1/stream?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
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

Response:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_at": "2025-12-14T10:30:00.000Z"
  }
}
```

### Authorization

After authentication, all operations are subject to RBAC (Role-Based Access Control). Users can only:
- Query tables they have `SELECT` permission on
- Subscribe to metrics if they have `MONITOR` privilege
- Execute queries based on their role permissions

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
  "timestamp": "2025-12-13T10:30:00.000Z",
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

#### 1. Batch Multiple Queries

Instead of:
```javascript
// Bad: Multiple round trips
ws.send(JSON.stringify({ type: 'query', payload: { sql: 'SELECT 1' } }));
ws.send(JSON.stringify({ type: 'query', payload: { sql: 'SELECT 2' } }));
```

Do:
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

#### 2. Use Compression

Enable compression for large messages (handled by the browser/client library):

```javascript
// Browser automatically handles compression if server supports it
// For Node.js, use ws library with perMessageDeflate option
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:8080/api/v1/stream', {
  perMessageDeflate: true
});
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

### Resource Management

#### 1. Clean Up Subscriptions

```javascript
window.addEventListener('beforeunload', () => {
  // Clean up before page unload
  ws.close(1000, 'Page unload');
});
```

#### 2. Monitor Memory Usage

```javascript
const messageHandlers = new Map();

function subscribe(eventType, handler) {
  if (!messageHandlers.has(eventType)) {
    messageHandlers.set(eventType, new Set());
  }
  messageHandlers.get(eventType).add(handler);

  // Return unsubscribe function
  return () => {
    const handlers = messageHandlers.get(eventType);
    handlers.delete(handler);
    if (handlers.size === 0) {
      messageHandlers.delete(eventType);
    }
  };
}
```

---

## Security Considerations

### Transport Layer Security

#### Always Use WSS in Production

```javascript
// Production: Use encrypted connection
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/api/v1/stream`);
```

**Configuration**:
- Enable TLS 1.2 or higher
- Use strong cipher suites
- Implement certificate validation
- Enable HSTS (HTTP Strict Transport Security)

### Authentication Security

#### Token Management

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

#### Authorization Checks

- Validate permissions on server side for every message
- Don't trust client-side validation
- Implement fine-grained RBAC
- Log all security-relevant events

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

Client-side best practice:
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

### Data Privacy

- Never log sensitive data in messages
- Implement data masking for PII
- Use Virtual Private Database (VPD) policies
- Enable audit logging for compliance

---

## Configuration Options

### Server-Side Configuration

Configure in `ApiConfig`:

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

## Code Examples

### Complete Client Implementation (JavaScript)

See [examples/websocket_client.js](../examples/websocket_client.js) (placeholder)

### Complete Client Implementation (Rust)

See [examples/websocket_client.rs](../examples/websocket_client.rs)

### Complete Client Implementation (Python)

See [examples/websocket_client.py](../examples/websocket_client.py)

### React Hook Example

```typescript
// Frontend integration using React
import { useWebSocket } from '../contexts/WebSocketContext';

function MyComponent() {
  const { connectionState, subscribe, send } = useWebSocket();

  useEffect(() => {
    const unsubscribe = subscribe('data', (payload) => {
      console.log('Received data:', payload);
    });

    return unsubscribe;
  }, [subscribe]);

  const executeQuery = () => {
    send('query', {
      sql: 'SELECT * FROM users',
      streaming: true
    });
  };

  return (
    <div>
      <p>Connection: {connectionState}</p>
      <button onClick={executeQuery}>Execute Query</button>
    </div>
  );
}
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

- [WebSocket RFC 6455](https://datatracker.ietf.org/doc/html/rfc6455)
- [GraphQL over WebSocket Protocol](https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md)
- [MDN WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
- [RustyDB API Reference](./API_REFERENCE.md)
- [RustyDB Security Architecture](./SECURITY_ARCHITECTURE.md)

---

## Support

For issues or questions:
- GitHub Issues: https://github.com/rustydb/rustydb/issues
- Documentation: https://docs.rustydb.com
- Community: https://community.rustydb.com

---

*Last Updated: 2025-12-13*
