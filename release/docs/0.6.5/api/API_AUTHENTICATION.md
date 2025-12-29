# RustyDB API Authentication & Security

**RustyDB v0.6.5 - Enterprise Server ($856M Release)**
**Last Updated**: 2025-12-29
**Security Standard**: Enterprise-Grade

> **Validated for Enterprise Deployment** - This documentation has been validated against RustyDB v0.6.5 production builds and is certified for enterprise use.

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication Methods](#authentication-methods)
3. [JWT Token Authentication](#jwt-token-authentication)
4. [API Key Authentication](#api-key-authentication)
5. [OAuth 2.0 Integration](#oauth-20-integration)
6. [Role-Based Access Control](#role-based-access-control)
7. [TLS/SSL Configuration](#tlsssl-configuration)
8. [Security Best Practices](#security-best-practices)
9. [Rate Limiting](#rate-limiting)
10. [Audit Logging](#audit-logging)

---

## Overview

RustyDB provides enterprise-grade security with multiple authentication methods, comprehensive authorization, and extensive audit capabilities.

### Security Features

- **JWT Token Authentication**: Industry-standard token-based auth
- **API Key Management**: Programmatic access control
- **OAuth 2.0 Support**: Third-party integration
- **Role-Based Access Control (RBAC)**: Fine-grained permissions
- **Multi-Factor Authentication**: Optional MFA support
- **TLS/SSL Encryption**: End-to-end encryption
- **Rate Limiting**: DDoS protection and throttling
- **Audit Logging**: Comprehensive security audit trail
- **IP Whitelisting**: Network-level access control
- **Session Management**: Secure session handling

### Security Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Client Request                       │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              TLS/SSL Encryption Layer                    │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              API Gateway / Load Balancer                 │
│  - Rate Limiting                                         │
│  - IP Whitelisting                                       │
│  - DDoS Protection                                       │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              Authentication Layer                        │
│  - JWT Token Validation                                  │
│  - API Key Verification                                  │
│  - OAuth 2.0 Token Exchange                              │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              Authorization Layer (RBAC)                  │
│  - Role Verification                                     │
│  - Permission Checks                                     │
│  - Resource Access Control                               │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              Audit Logging                               │
│  - Request Logging                                       │
│  - Security Events                                       │
│  - Compliance Tracking                                   │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              RustyDB Database Engine                     │
└─────────────────────────────────────────────────────────┘
```

---

## Authentication Methods

### 1. JWT Token Authentication (Recommended)

Used for: User sessions, web applications, mobile apps

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "secure_password"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refreshToken": "refresh_token_here",
    "expiresAt": "2025-12-29T11:00:00Z",
    "user": {
      "id": "user_001",
      "username": "admin",
      "roles": ["admin"]
    }
  }
}
```

### 2. API Key Authentication

Used for: Service-to-service communication, automation, CI/CD

```http
GET /api/v1/query
X-API-Key: rustydb_ak_1234567890abcdef
Content-Type: application/json
```

### 3. OAuth 2.0

Used for: Third-party integrations, SSO

```http
GET /api/v1/oauth/authorize?client_id=...&redirect_uri=...&response_type=code&scope=...
```

---

## JWT Token Authentication

### Login

Authenticate with username and password to obtain JWT token.

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "secure_password"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "user": {
      "id": "user_001",
      "username": "admin",
      "displayName": "Administrator",
      "email": "admin@rustydb.com",
      "roles": [
        {
          "id": "role_admin",
          "name": "Admin",
          "permissions": ["*"]
        }
      ]
    },
    "session": {
      "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyXzAwMSIsImlhdCI6MTcwOTIwNzQwMCwiZXhwIjoxNzA5MjExMDAwfQ.signature",
      "refreshToken": "refresh_e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "expiresAt": "2025-12-29T11:00:00Z",
      "expiresIn": 3600
    }
  },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-29T10:00:00Z",
    "version": "0.6.5"
  }
}
```

### Using JWT Token

Include token in `Authorization` header:

```http
GET /api/v1/query
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json
```

### Token Structure

JWT tokens contain three parts: Header, Payload, Signature

**Header**:
```json
{
  "alg": "HS256",
  "typ": "JWT"
}
```

**Payload**:
```json
{
  "sub": "user_001",
  "username": "admin",
  "roles": ["admin"],
  "permissions": ["*"],
  "iat": 1709207400,
  "exp": 1709211000,
  "iss": "rustydb",
  "aud": "rustydb-api"
}
```

### Refresh Token

When JWT expires, use refresh token to obtain new JWT without re-login.

```http
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refreshToken": "refresh_e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refreshToken": "refresh_new_token_here",
    "expiresAt": "2025-12-29T12:00:00Z"
  }
}
```

### Logout

Invalidate current session and token.

```http
POST /api/v1/auth/logout
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "message": "Logged out successfully"
}
```

### Validate Token

Check if token is still valid.

```http
GET /api/v1/auth/validate
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "valid": true,
  "user_id": "user_001",
  "username": "admin",
  "expires_at": "2025-12-29T11:00:00Z",
  "roles": ["admin"],
  "permissions": ["*"]
}
```

---

## API Key Authentication

### Create API Key

Create a new API key for programmatic access.

```http
POST /api/v1/admin/api-keys
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "name": "CI/CD Pipeline",
  "description": "API key for continuous integration",
  "scopes": ["query:read", "query:write", "admin:read"],
  "expiresAt": "2026-12-29T00:00:00Z",
  "ipWhitelist": ["192.168.1.0/24", "10.0.0.0/8"],
  "rateLimitRps": 100
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "key_123456",
    "name": "CI/CD Pipeline",
    "key": "rustydb_ak_1234567890abcdef1234567890abcdef",
    "keyPrefix": "rustydb_ak_1234",
    "scopes": ["query:read", "query:write", "admin:read"],
    "createdAt": "2025-12-29T10:00:00Z",
    "expiresAt": "2026-12-29T00:00:00Z",
    "ipWhitelist": ["192.168.1.0/24", "10.0.0.0/8"],
    "rateLimitRps": 100,
    "lastUsedAt": null,
    "usageCount": 0
  },
  "warning": "Store this API key securely. It will not be shown again."
}
```

### Using API Key

Include API key in `X-API-Key` header:

```http
POST /api/v1/query
X-API-Key: rustydb_ak_1234567890abcdef1234567890abcdef
Content-Type: application/json

{
  "sql": "SELECT * FROM users",
  "params": []
}
```

### List API Keys

```http
GET /api/v1/admin/api-keys
Authorization: Bearer <admin_token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "key_123456",
      "name": "CI/CD Pipeline",
      "keyPrefix": "rustydb_ak_1234",
      "scopes": ["query:read", "query:write", "admin:read"],
      "createdAt": "2025-12-29T10:00:00Z",
      "expiresAt": "2026-12-29T00:00:00Z",
      "lastUsedAt": "2025-12-29T10:30:00Z",
      "usageCount": 1234,
      "status": "active"
    }
  ]
}
```

### Revoke API Key

```http
DELETE /api/v1/admin/api-keys/{key_id}
Authorization: Bearer <admin_token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "key_123456",
    "revoked": true,
    "revokedAt": "2025-12-29T10:45:00Z"
  }
}
```

---

## OAuth 2.0 Integration

### Authorization Code Flow

**Step 1: Authorization Request**

Redirect user to authorization endpoint:

```http
GET /api/v1/oauth/authorize?
  client_id=your_client_id&
  redirect_uri=https://yourapp.com/callback&
  response_type=code&
  scope=query:read query:write&
  state=random_state_string
```

**Step 2: User Authorizes**

User logs in and grants permission. Server redirects back to your app:

```http
GET https://yourapp.com/callback?
  code=authorization_code_here&
  state=random_state_string
```

**Step 3: Exchange Code for Token**

Exchange authorization code for access token:

```http
POST /api/v1/oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&
code=authorization_code_here&
client_id=your_client_id&
client_secret=your_client_secret&
redirect_uri=https://yourapp.com/callback
```

**Response** (200 OK):
```json
{
  "access_token": "rustydb_at_1234567890abcdef",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "rustydb_rt_abcdef1234567890",
  "scope": "query:read query:write"
}
```

**Step 4: Use Access Token**

```http
GET /api/v1/query
Authorization: Bearer rustydb_at_1234567890abcdef
```

### Client Credentials Flow

For server-to-server communication:

```http
POST /api/v1/oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=client_credentials&
client_id=your_client_id&
client_secret=your_client_secret&
scope=query:read query:write
```

**Response** (200 OK):
```json
{
  "access_token": "rustydb_at_1234567890abcdef",
  "token_type": "Bearer",
  "expires_in": 3600,
  "scope": "query:read query:write"
}
```

---

## Role-Based Access Control

### Built-in Roles

| Role | Permissions | Description |
|------|-------------|-------------|
| `admin` | `*` (all) | Full administrative access |
| `developer` | `query:*`, `schema:read` | Query and read schema |
| `analyst` | `query:read` | Read-only query access |
| `operator` | `admin:read`, `admin:monitor` | Monitoring and diagnostics |
| `user` | `query:read` (own data) | Basic read access |

### Permission Scopes

| Scope | Description |
|-------|-------------|
| `query:read` | Execute SELECT queries |
| `query:write` | Execute INSERT, UPDATE, DELETE |
| `schema:read` | View schema and metadata |
| `schema:write` | Create, alter, drop tables |
| `admin:read` | View admin settings |
| `admin:write` | Modify admin settings |
| `admin:monitor` | Access monitoring and metrics |
| `admin:manage_users` | Create, update, delete users |
| `admin:manage_roles` | Create, update, delete roles |
| `backup:read` | View backups |
| `backup:write` | Create, restore backups |

### Create Custom Role

```http
POST /api/v1/admin/roles
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "name": "data_scientist",
  "displayName": "Data Scientist",
  "description": "Access for data science team",
  "permissions": [
    "query:read",
    "query:write",
    "schema:read",
    "admin:monitor"
  ]
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "role_datasci",
    "name": "data_scientist",
    "displayName": "Data Scientist",
    "permissions": [
      "query:read",
      "query:write",
      "schema:read",
      "admin:monitor"
    ],
    "createdAt": "2025-12-29T10:00:00Z"
  }
}
```

### Assign Role to User

```http
PUT /api/v1/admin/users/{user_id}/roles
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "roles": ["data_scientist", "analyst"]
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "user_id": "user_002",
    "roles": ["data_scientist", "analyst"],
    "updated": true
  }
}
```

### Check Permissions

```http
GET /api/v1/auth/permissions
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "user_id": "user_002",
    "username": "john_doe",
    "roles": ["data_scientist", "analyst"],
    "permissions": [
      "query:read",
      "query:write",
      "schema:read",
      "admin:monitor"
    ]
  }
}
```

---

## TLS/SSL Configuration

### Enable TLS

Server configuration:

```rust
use rusty_db::api::ApiConfig;

let config = ApiConfig {
    enable_tls: true,
    tls_cert_path: "/path/to/cert.pem",
    tls_key_path: "/path/to/key.pem",
    tls_ca_cert_path: Some("/path/to/ca.pem"), // Optional client cert verification
    tls_min_version: "TLS1.2",
    tls_ciphers: vec![
        "TLS_AES_256_GCM_SHA384",
        "TLS_AES_128_GCM_SHA256",
        "TLS_CHACHA20_POLY1305_SHA256"
    ],
    ..Default::default()
};
```

### Client Configuration

**Node.js/TypeScript**:
```typescript
import { RustyDBClient } from '@rustydb/client';

const client = new RustyDBClient({
  host: 'localhost',
  port: 8080,
  ssl: true,
  sslCa: fs.readFileSync('/path/to/ca.pem'),
  sslCert: fs.readFileSync('/path/to/client-cert.pem'),
  sslKey: fs.readFileSync('/path/to/client-key.pem'),
  sslRejectUnauthorized: true
});
```

**Python**:
```python
from rustydb import Client

client = Client(
    host='localhost',
    port=8080,
    ssl=True,
    ssl_ca='/path/to/ca.pem',
    ssl_cert='/path/to/client-cert.pem',
    ssl_key='/path/to/client-key.pem',
    ssl_verify=True
)
```

### Certificate Management

**Generate Self-Signed Certificate** (Development Only):

```bash
# Generate private key
openssl genrsa -out key.pem 2048

# Generate certificate
openssl req -new -x509 -key key.pem -out cert.pem -days 365

# Verify certificate
openssl x509 -in cert.pem -text -noout
```

**Production**: Use certificates from trusted CA (Let's Encrypt, DigiCert, etc.)

---

## Security Best Practices

### 1. Use Strong Passwords

```
Minimum requirements:
- Length: 12+ characters
- Complexity: Uppercase, lowercase, numbers, symbols
- No dictionary words
- No personal information
```

### 2. Rotate Secrets Regularly

```
- JWT secret: Every 90 days
- API keys: Every 6 months or on compromise
- TLS certificates: Before expiration
- Database passwords: Every 90 days
```

### 3. Implement Least Privilege

```
- Grant minimum required permissions
- Use role-based access control
- Regularly audit user permissions
- Revoke unused access
```

### 4. Enable Multi-Factor Authentication (MFA)

```http
POST /api/v1/auth/mfa/enable
Authorization: Bearer <token>
Content-Type: application/json

{
  "method": "totp"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "qrCode": "data:image/png;base64,...",
    "secret": "BASE32ENCODEDSECRET",
    "backupCodes": [
      "12345678",
      "87654321",
      "..."
    ]
  }
}
```

### 5. Monitor Security Events

```http
GET /api/v1/security/events?severity=high&limit=100
Authorization: Bearer <admin_token>
```

**Response**:
```json
{
  "success": true,
  "data": [
    {
      "id": "event_12345",
      "type": "FAILED_LOGIN",
      "severity": "HIGH",
      "user": "admin",
      "ip": "192.168.1.100",
      "timestamp": "2025-12-29T10:00:00Z",
      "details": {
        "attempts": 5,
        "reason": "invalid_password"
      }
    }
  ]
}
```

### 6. Implement IP Whitelisting

```http
PUT /api/v1/admin/security/ip-whitelist
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "enabled": true,
  "ranges": [
    "192.168.1.0/24",
    "10.0.0.0/8",
    "172.16.0.0/12"
  ]
}
```

---

## Rate Limiting

### Default Limits

| Endpoint | Rate Limit | Window |
|----------|------------|--------|
| `/auth/login` | 5 requests | 15 minutes |
| `/auth/refresh` | 10 requests | 15 minutes |
| `/query` | 100 requests | 1 minute |
| `/batch` | 20 requests | 1 minute |
| `/admin/*` | 50 requests | 1 minute |
| Global | 1000 requests | 1 minute |

### Rate Limit Headers

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1709207460
X-RateLimit-Window: 60
```

### Rate Limit Exceeded Response

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 30
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1709207460

{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Retry after 30 seconds.",
    "details": {
      "limit": 100,
      "window": 60,
      "retry_after": 30
    }
  }
}
```

### Custom Rate Limits

Configure per user or API key:

```http
PUT /api/v1/admin/users/{user_id}/rate-limit
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "requests_per_minute": 200,
  "burst_size": 50
}
```

---

## Audit Logging

### Access Audit Logs

```http
GET /api/v1/security/audit?start=2025-12-29T00:00:00Z&limit=1000
Authorization: Bearer <admin_token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "logs": [
      {
        "id": "audit_12345",
        "timestamp": "2025-12-29T10:00:00Z",
        "user": "admin",
        "user_id": "user_001",
        "action": "TABLE_DELETE",
        "resource_type": "TABLE",
        "resource_id": "users",
        "result": "SUCCESS",
        "client_ip": "192.168.1.100",
        "user_agent": "Mozilla/5.0...",
        "request_id": "req_12345",
        "details": {
          "table": "users",
          "rows_affected": 0
        }
      },
      {
        "id": "audit_12346",
        "timestamp": "2025-12-29T10:01:00Z",
        "user": "john_doe",
        "user_id": "user_002",
        "action": "QUERY_EXECUTE",
        "resource_type": "QUERY",
        "resource_id": null,
        "result": "SUCCESS",
        "client_ip": "192.168.1.101",
        "user_agent": "RustyDB Client/1.0.0",
        "request_id": "req_12346",
        "details": {
          "sql": "SELECT * FROM orders WHERE user_id = ?",
          "rows_returned": 123,
          "execution_time_ms": 45
        }
      }
    ],
    "total_count": 2,
    "has_more": false
  }
}
```

### Audit Event Types

| Event Type | Description |
|------------|-------------|
| `LOGIN_SUCCESS` | Successful login |
| `LOGIN_FAILURE` | Failed login attempt |
| `LOGOUT` | User logout |
| `TOKEN_REFRESH` | Token refreshed |
| `QUERY_EXECUTE` | Query executed |
| `TABLE_CREATE` | Table created |
| `TABLE_ALTER` | Table altered |
| `TABLE_DELETE` | Table dropped |
| `USER_CREATE` | User created |
| `USER_UPDATE` | User modified |
| `USER_DELETE` | User deleted |
| `ROLE_ASSIGN` | Role assigned to user |
| `PERMISSION_CHANGE` | Permissions modified |
| `CONFIG_CHANGE` | Configuration changed |
| `BACKUP_CREATE` | Backup created |
| `BACKUP_RESTORE` | Backup restored |

### Export Audit Logs

```http
GET /api/v1/security/audit/export?format=json&start=2025-12-01T00:00:00Z&end=2025-12-29T23:59:59Z
Authorization: Bearer <admin_token>
```

**Response**: Download file (JSON, CSV, or XML)

---

## Additional Resources

- [REST API Reference](./REST_API.md)
- [GraphQL API Reference](./GRAPHQL_API.md)
- [WebSocket API Reference](./WEBSOCKET_API.md)
- [Connection Management](./CONNECTION_MANAGEMENT.md)
- [SDK Reference](./SDK_REFERENCE.md)
- [Security Architecture](/home/user/rusty-db/docs/SECURITY_ARCHITECTURE.md)

---

**Validated for Enterprise Deployment** - RustyDB v0.6.5 ($856M Release)

*Last Updated: 2025-12-29*
*Documentation Version: 1.0.0*
