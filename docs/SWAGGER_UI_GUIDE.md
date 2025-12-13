# Swagger UI Guide

**Version**: 1.0.0
**Last Updated**: 2025-12-13
**Status**: In Development

## Table of Contents

1. [Overview](#overview)
2. [Accessing Swagger UI](#accessing-swagger-ui)
3. [Getting Started](#getting-started)
4. [Authentication in Swagger](#authentication-in-swagger)
5. [Testing Endpoints Interactively](#testing-endpoints-interactively)
6. [OpenAPI Specification](#openapi-specification)
7. [Customization Options](#customization-options)
8. [Advanced Features](#advanced-features)
9. [Troubleshooting](#troubleshooting)

---

## Overview

RustyDB provides interactive API documentation through Swagger UI, powered by OpenAPI 3.0 specifications. Swagger UI enables developers to:

- Explore all available API endpoints
- Understand request/response schemas
- Test APIs directly from the browser
- Generate client code
- Export API specifications

### What is Swagger UI?

Swagger UI is an open-source tool that generates interactive API documentation from OpenAPI specifications. It provides:

- **Visual Interface**: Browse endpoints by category
- **Interactive Testing**: Execute API calls directly
- **Schema Visualization**: View request/response models
- **Authentication Support**: Test authenticated endpoints
- **Code Generation**: Generate client SDKs

### Technology Stack

- **OpenAPI**: 3.0.3 specification
- **utoipa**: Rust OpenAPI code generator (v5.0)
- **utoipa-swagger-ui**: Axum integration (v9.0)
- **Swagger UI**: Frontend interface (latest)

---

## Accessing Swagger UI

### Development Environment

**URL**: `http://localhost:8080/swagger-ui`

**Prerequisites**:
1. RustyDB server running
2. Swagger UI enabled in configuration
3. Browser with JavaScript enabled

```bash
# Start RustyDB server
cargo run --bin rusty-db-server

# Server logs will show:
# INFO REST API server listening on 0.0.0.0:8080
# INFO Swagger UI available at http://localhost:8080/swagger-ui
```

### Production Environment

**URL**: `https://your-domain.com/swagger-ui`

**Security Considerations**:
- Use HTTPS (required)
- Implement IP whitelisting
- Require authentication
- Consider disabling in production (enable only for internal teams)

### Configuration

Enable/disable Swagger UI in `ApiConfig`:

```rust
use rusty_db::api::ApiConfig;

let config = ApiConfig {
    enable_swagger: true,  // Set to false to disable
    swagger_path: "/swagger-ui",  // Custom path
    ..Default::default()
};
```

---

## Getting Started

### First Time Setup

#### 1. Access Swagger UI

Open your browser and navigate to:
```
http://localhost:8080/swagger-ui
```

You should see the Swagger UI interface with:
- API title: "RustyDB REST API"
- Version information
- List of endpoint categories

#### 2. Browse API Categories

Swagger UI organizes endpoints by category:

```
┌─────────────────────────────────────────────┐
│ RustyDB REST API - v1.0.0                   │
├─────────────────────────────────────────────┤
│ ▼ Authentication                            │
│   POST /api/v1/auth/login                   │
│   POST /api/v1/auth/logout                  │
│   POST /api/v1/auth/refresh                 │
│                                             │
│ ▼ Database Operations                       │
│   POST /api/v1/query                        │
│   POST /api/v1/batch                        │
│   GET  /api/v1/tables/{name}                │
│                                             │
│ ▼ Administration                            │
│   GET  /api/v1/admin/health                 │
│   GET  /api/v1/admin/config                 │
│   POST /api/v1/admin/backup                 │
└─────────────────────────────────────────────┘
```

#### 3. Explore an Endpoint

Click on any endpoint to expand it and see:
- HTTP method and path
- Description
- Parameters (path, query, body)
- Request schema
- Response schemas
- Example values

---

## Authentication in Swagger

Most RustyDB endpoints require authentication. Swagger UI provides multiple ways to authenticate.

### Method 1: Authorize Button (Recommended)

#### Step 1: Click the "Authorize" button

Located at the top-right of the Swagger UI interface.

#### Step 2: Enter your JWT token

```
Available authorizations

bearerAuth (http, Bearer)
┌─────────────────────────────────────────────┐
│ Value:                                      │
│ ┌─────────────────────────────────────────┐│
│ │ eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...││
│ └─────────────────────────────────────────┘│
└─────────────────────────────────────────────┘

[Authorize]  [Close]
```

#### Step 3: Click "Authorize"

The token is now stored and will be included in all API requests as:
```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

#### Step 4: Verify

Endpoints now show a lock icon 🔒 indicating authentication is configured.

### Method 2: Get Token via Login Endpoint

If you don't have a token:

#### 1. Expand the Login Endpoint

Navigate to:
```
▼ Authentication
  ▼ POST /api/v1/auth/login
```

#### 2. Click "Try it out"

#### 3. Enter Credentials

Modify the request body:
```json
{
  "username": "admin",
  "password": "your_password"
}
```

#### 4. Click "Execute"

#### 5. Copy the Token

From the response:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_at": "2025-12-14T10:30:00Z"
  }
}
```

#### 6. Use "Authorize" Button

Paste the token and authorize.

### Method 3: Direct Header Input

For individual endpoint testing, you can manually add headers:

1. Expand the endpoint
2. Click "Try it out"
3. Scroll to "Headers" section (if available)
4. Add: `Authorization: Bearer YOUR_TOKEN`

---

## Testing Endpoints Interactively

### Basic Query Example

#### 1. Navigate to Query Endpoint

```
▼ Database Operations
  ▼ POST /api/v1/query
```

#### 2. Click "Try it out"

The request body becomes editable.

#### 3. Modify Request Body

```json
{
  "sql": "SELECT * FROM users WHERE age > 18",
  "params": [],
  "timeout_secs": 30
}
```

#### 4. Click "Execute"

Swagger sends the request and displays:

**Request URL**:
```
http://localhost:8080/api/v1/query
```

**Request Headers**:
```
Content-Type: application/json
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Response Body**:
```json
{
  "success": true,
  "data": {
    "rows": [
      ["alice@example.com", "Alice", 25],
      ["bob@example.com", "Bob", 30]
    ],
    "columns": ["email", "name", "age"],
    "rows_affected": 2,
    "execution_time_ms": 12
  }
}
```

**Response Code**: `200 OK`

### Testing with Path Parameters

For endpoints like `GET /api/v1/tables/{name}`:

#### 1. Expand the Endpoint

```
▼ Database Operations
  ▼ GET /api/v1/tables/{name}
```

#### 2. Click "Try it out"

#### 3. Enter Path Parameter

```
Parameters
┌─────────────────────────────────────────────┐
│ name * (required)                           │
│ string (path)                               │
│ ┌─────────────────────────────────────────┐│
│ │ users                                   ││
│ └─────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
```

#### 4. Click "Execute"

Request sent to: `GET /api/v1/tables/users`

### Testing with Query Parameters

For endpoints like `GET /api/v1/logs?level=error&limit=10`:

#### 1. Expand the Endpoint

```
▼ Monitoring
  ▼ GET /api/v1/logs
```

#### 2. Click "Try it out"

#### 3. Fill Query Parameters

```
Parameters
┌─────────────────────────────────────────────┐
│ level                                       │
│ string (query)                              │
│ ┌─────────────────────────────────────────┐│
│ │ error                                   ││
│ └─────────────────────────────────────────┘│
│                                             │
│ limit                                       │
│ integer (query)                             │
│ ┌─────────────────────────────────────────┐│
│ │ 10                                      ││
│ └─────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
```

#### 4. Click "Execute"

Request sent to: `GET /api/v1/logs?level=error&limit=10`

### Testing File Uploads (Future)

For multipart/form-data endpoints:

```
POST /api/v1/backup/upload

┌─────────────────────────────────────────────┐
│ file *                                      │
│ file (formData)                             │
│ [Choose File] backup.sql                    │
│                                             │
│ metadata                                    │
│ string (formData)                           │
│ ┌─────────────────────────────────────────┐│
│ │ {"description": "Daily backup"}         ││
│ └─────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
```

---

## OpenAPI Specification

### Viewing the Specification

#### JSON Format

**URL**: `http://localhost:8080/api-docs/openapi.json`

```bash
curl http://localhost:8080/api-docs/openapi.json > openapi.json
```

#### YAML Format

**URL**: `http://localhost:8080/api-docs/openapi.yaml`

```bash
curl http://localhost:8080/api-docs/openapi.yaml > openapi.yaml
```

### Specification Structure

```json
{
  "openapi": "3.0.3",
  "info": {
    "title": "RustyDB REST API",
    "version": "1.0.0",
    "description": "Enterprise-grade database management system",
    "contact": {
      "name": "RustyDB Team",
      "url": "https://github.com/rustydb/rustydb"
    },
    "license": {
      "name": "MIT"
    }
  },
  "servers": [
    {
      "url": "http://localhost:8080",
      "description": "Development server"
    }
  ],
  "paths": {
    "/api/v1/query": {
      "post": {
        "tags": ["Database Operations"],
        "summary": "Execute SQL query",
        "operationId": "execute_query",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/QueryRequest"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Query executed successfully",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/QueryResponse"
                }
              }
            }
          }
        },
        "security": [
          {
            "bearerAuth": []
          }
        ]
      }
    }
  },
  "components": {
    "schemas": {
      "QueryRequest": {
        "type": "object",
        "required": ["sql"],
        "properties": {
          "sql": {
            "type": "string",
            "description": "SQL query to execute"
          },
          "params": {
            "type": "array",
            "items": {
              "type": "string"
            }
          }
        }
      }
    },
    "securitySchemes": {
      "bearerAuth": {
        "type": "http",
        "scheme": "bearer",
        "bearerFormat": "JWT"
      }
    }
  }
}
```

### Using the Specification

#### Generate Client SDKs

```bash
# Install OpenAPI Generator
npm install -g @openapitools/openapi-generator-cli

# Generate TypeScript client
openapi-generator-cli generate \
  -i http://localhost:8080/api-docs/openapi.json \
  -g typescript-axios \
  -o ./client/typescript

# Generate Python client
openapi-generator-cli generate \
  -i http://localhost:8080/api-docs/openapi.json \
  -g python \
  -o ./client/python

# Generate Rust client
openapi-generator-cli generate \
  -i http://localhost:8080/api-docs/openapi.json \
  -g rust \
  -o ./client/rust
```

#### Import into Postman

1. Open Postman
2. Click "Import"
3. Select "Link" tab
4. Enter: `http://localhost:8080/api-docs/openapi.json`
5. Click "Import"

All endpoints are now available in Postman.

#### Import into Insomnia

1. Open Insomnia
2. Click "Create" → "Import"
3. Select "From URL"
4. Enter: `http://localhost:8080/api-docs/openapi.json`
5. Click "Fetch and Import"

---

## Customization Options

### Server-Side Customization

#### Custom API Information

```rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "RustyDB REST API",
        version = "1.0.0",
        description = "Enterprise-grade Oracle-compatible database",
        contact(
            name = "RustyDB Support",
            url = "https://rustydb.com/support",
            email = "support@rustydb.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Development"),
        (url = "https://api.rustydb.com", description = "Production")
    ),
    tags(
        (name = "Database Operations", description = "Core database operations"),
        (name = "Authentication", description = "User authentication"),
        (name = "Administration", description = "Admin operations")
    )
)]
struct ApiDoc;
```

#### Custom Swagger UI Configuration

```rust
use utoipa_swagger_ui::{SwaggerUi, Config};

let swagger_ui = SwaggerUi::new("/swagger-ui")
    .url("/api-docs/openapi.json", ApiDoc::openapi())
    .config(Config::default()
        .try_it_out_enabled(true)
        .filter(true)
        .show_extensions(true)
        .show_common_extensions(true)
    );
```

### Client-Side Customization

#### Custom Theme

Swagger UI supports custom CSS. Add to your server:

```css
/* custom-swagger.css */
.swagger-ui .topbar {
  background-color: #1a1a2e;
}

.swagger-ui .info .title {
  color: #16213e;
  font-size: 36px;
}

.swagger-ui .scheme-container {
  background: #f0f0f0;
}
```

Serve custom CSS:
```rust
.route("/swagger-ui/custom.css", get(serve_custom_css))
```

#### Disable Try-It-Out

For documentation-only mode:

```javascript
SwaggerUIBundle({
  url: "/api-docs/openapi.json",
  dom_id: '#swagger-ui',
  supportedSubmitMethods: []  // Disable all try-it-out
})
```

---

## Advanced Features

### Server Selection

Test against different environments:

```
Servers
┌─────────────────────────────────────────────┐
│ ▼ http://localhost:8080 - Development       │
│   https://staging.rustydb.com - Staging     │
│   https://api.rustydb.com - Production      │
└─────────────────────────────────────────────┘
```

All requests will be sent to the selected server.

### Response Examples

View multiple response examples:

```
Responses
├─ 200 OK
│  └─ Example: Success
│     {
│       "success": true,
│       "data": { ... }
│     }
│
├─ 400 Bad Request
│  └─ Example: Invalid SQL
│     {
│       "success": false,
│       "error": "Syntax error in SQL"
│     }
│
└─ 401 Unauthorized
   └─ Example: Missing Token
      {
        "success": false,
        "error": "Authentication required"
      }
```

### Schema Validation

Swagger UI validates requests against schemas:

```json
// Invalid request (missing required field)
{
  "params": []
}
```

**Validation Error**:
```
⚠ Request body is required but not present
⚠ Property 'sql' is required
```

### Download Response

Click "Download" button to save response:
- JSON format
- Pretty-printed
- Includes metadata

### Copy as cURL

Click "Copy" to get the equivalent cURL command:

```bash
curl -X POST "http://localhost:8080/api/v1/query" \
  -H "accept: application/json" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT * FROM users",
    "params": []
  }'
```

---

## Troubleshooting

### Swagger UI Not Loading

**Symptoms**: Blank page or 404 error

**Solutions**:
1. Verify server is running: `curl http://localhost:8080/api/v1/admin/health`
2. Check configuration: `enable_swagger: true`
3. Verify URL: `http://localhost:8080/swagger-ui` (note the hyphen)
4. Check browser console for JavaScript errors
5. Clear browser cache and reload

### Authentication Not Working

**Symptoms**: 401 errors despite using Authorize button

**Solutions**:
1. Verify token format: `Bearer <token>`
2. Check token expiration
3. Refresh token via `/api/v1/auth/refresh`
4. Re-login to get new token
5. Verify user has required permissions

### Request Timeout

**Symptoms**: Request takes too long, returns timeout error

**Solutions**:
1. Increase timeout in request body: `"timeout_secs": 60`
2. Optimize SQL query
3. Check server resources
4. Review server logs for errors

### CORS Errors

**Symptoms**: "CORS policy" error in browser console

**Solutions**:
1. Enable CORS in server config: `enable_cors: true`
2. Add origin to whitelist: `cors_origins: ["http://localhost:3000"]`
3. Check preflight OPTIONS requests
4. Verify protocol (http vs https)

### OpenAPI Spec Not Found

**Symptoms**: Cannot download OpenAPI JSON/YAML

**Solutions**:
1. Verify URL: `http://localhost:8080/api-docs/openapi.json`
2. Check server logs for errors
3. Ensure utoipa dependencies are included
4. Rebuild with `cargo build --release`

### Invalid Request Body

**Symptoms**: Validation errors for valid JSON

**Solutions**:
1. Check JSON syntax (trailing commas, quotes)
2. Verify required fields are present
3. Check data types (string vs number)
4. Review schema in Swagger UI
5. Use "Example Value" as template

---

## Best Practices

### Documentation

1. **Add Descriptions**: Use clear, concise descriptions for all endpoints
2. **Provide Examples**: Include realistic request/response examples
3. **Document Errors**: List all possible error codes and meanings
4. **Version Control**: Track OpenAPI spec changes in git

### Testing

1. **Test Happy Paths**: Verify successful requests work as expected
2. **Test Error Cases**: Verify proper error handling
3. **Test Edge Cases**: Empty arrays, null values, special characters
4. **Test Performance**: Monitor response times

### Security

1. **Disable in Production**: Or restrict access to internal networks
2. **Use Authentication**: Require login for Swagger UI access
3. **Sanitize Examples**: Don't include real credentials or sensitive data
4. **Regular Updates**: Keep Swagger UI dependencies up to date

### Maintenance

1. **Sync with Code**: Ensure OpenAPI spec matches implementation
2. **Automated Tests**: Validate spec against running server
3. **Documentation Reviews**: Regular reviews for accuracy
4. **Version Spec**: Use semantic versioning for API changes

---

## Next Steps

1. **Explore Endpoints**: Browse all available API endpoints
2. **Test Authentication**: Practice login and token management
3. **Execute Queries**: Try database operations
4. **Generate Client**: Create SDK for your preferred language
5. **Read API Reference**: Detailed endpoint documentation at [API_REFERENCE.md](./API_REFERENCE.md)

---

## Additional Resources

- [OpenAPI Specification](https://spec.openapis.org/oas/v3.0.3)
- [Swagger UI Documentation](https://swagger.io/docs/open-source-tools/swagger-ui/)
- [utoipa Documentation](https://docs.rs/utoipa/)
- [RustyDB API Reference](./API_REFERENCE.md)
- [RustyDB WebSocket Guide](./WEBSOCKET_INTEGRATION.md)

---

## Support

For issues or questions:
- GitHub Issues: https://github.com/rustydb/rustydb/issues
- Documentation: https://docs.rustydb.com
- Community: https://community.rustydb.com

---

*Last Updated: 2025-12-13*
