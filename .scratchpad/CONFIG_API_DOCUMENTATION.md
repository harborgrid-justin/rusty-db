# RustyDB Configuration API Documentation

**Version**: 0.3.001
**Instance Layout Spec**: v1.0
**Date**: 2025-12-14

## Overview

RustyDB provides comprehensive configuration management through both REST API and GraphQL interfaces. This document covers all configuration endpoints, types, and usage patterns.

### Architecture

The configuration system follows Instance Layout Spec v1.0:

```
<INSTANCE_ROOT>/
├── conf/
│   ├── rustydb.toml          # Base configuration
│   └── overrides.d/          # Override files (lexicographic)
├── data/
│   └── meta/                 # Instance metadata
│       ├── layout-version
│       ├── instance-id
│       ├── created-at
│       ├── data-format-version
│       ├── wal-format-version
│       ├── protocol-version
│       └── compat.json
├── logs/
├── run/
├── cache/
├── tmp/
├── backup/
└── diag/
```

---

## REST API Reference

### Base URL

```
http://<host>:54321/api/v1
```

### Configuration Endpoints

#### Get Full Configuration

```http
GET /api/v1/config
```

**Response** (200 OK):
```json
{
  "instance": {
    "name": "default",
    "instance_id": "550e8400-e29b-41d4-a716-446655440000",
    "description": ""
  },
  "paths": {
    "conf_dir": "conf",
    "data_dir": "data",
    "logs_dir": "logs",
    "run_dir": "run",
    "cache_dir": "cache",
    "tmp_dir": "tmp",
    "backup_dir": "backup",
    "diag_dir": "diag"
  },
  "server": {
    "listen_host": "127.0.0.1",
    "listen_port": 54321,
    "max_connections": 500,
    "idle_timeout_ms": 300000,
    "request_timeout_ms": 30000,
    "ipc_enabled": true,
    "ipc_path": "sockets"
  },
  "security": {
    "mode": "dev",
    "tls_enabled": false,
    "auth_mode": "none"
  },
  "tls": {
    "enabled": false,
    "cert_path": "secrets/tls/server.crt",
    "key_path": "secrets/tls/server.key",
    "min_version": "1.2",
    "require_client_cert": false
  },
  "logging": {
    "mode": "file",
    "format": "json",
    "level": "info",
    "audit_enabled": false,
    "rotate": true,
    "max_files": 10,
    "max_file_size_mb": 100
  },
  "storage": {
    "fsync": true,
    "sync_interval_ms": 1000,
    "page_size": 4096,
    "buffer_pool_pages": 1000
  },
  "wal": {
    "enabled": true,
    "dir": "wal",
    "max_segment_mb": 64,
    "checkpoint_interval_ms": 60000,
    "sync_mode": "local"
  },
  "cache": {
    "enabled": true,
    "max_size_mb": 512,
    "ml_enabled": true,
    "query_cache_enabled": true,
    "query_cache_ttl_ms": 60000
  },
  "metrics": {
    "enabled": true,
    "mode": "pull",
    "listen_host": "127.0.0.1",
    "listen_port": 9100,
    "path": "/metrics"
  },
  "diagnostics": {
    "write_build_info": true,
    "write_runtime_info": true,
    "max_log_bytes": 10485760,
    "core_dumps_enabled": false
  },
  "compat": {
    "fail_on_unsupported_layout": true,
    "fail_on_unsupported_data_format": true
  }
}
```

#### Get Individual Configuration Sections

```http
GET /api/v1/config/instance
GET /api/v1/config/paths
GET /api/v1/config/server
GET /api/v1/config/security
GET /api/v1/config/logging
GET /api/v1/config/storage
GET /api/v1/config/wal
GET /api/v1/config/cache
GET /api/v1/config/metrics
GET /api/v1/config/diagnostics
GET /api/v1/config/compat
```

#### Get Resolved Configuration

Returns configuration with all overrides applied.

```http
GET /api/v1/config/resolved
```

#### Reload Configuration

Hot-reload configuration from disk.

```http
PUT /api/v1/config/reload
```

**Response** (200 OK):
```json
{
  "success": true,
  "message": "Configuration reloaded successfully",
  "reloaded_at": 1734134400
}
```

### Metadata Endpoints

#### Get Instance Metadata

```http
GET /api/v1/metadata
```

**Response**:
```json
{
  "layout_version": "1.0",
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "created_at": "2025-12-14T00:00:00Z",
  "data_format_version": 2,
  "wal_format_version": 2,
  "protocol_version": 2,
  "last_upgraded_from": null
}
```

#### Get Version Information

```http
GET /api/v1/metadata/version
```

**Response**:
```json
{
  "binary_version": "0.3.001",
  "layout_version": "1.0",
  "data_format_version": 2,
  "wal_format_version": 2,
  "protocol_version": 2
}
```

---

## GraphQL API Reference

### Endpoint

```
POST /graphql
```

### Configuration Queries

```graphql
# Get full configuration
query {
  config {
    instance {
      name
      instanceId
      description
    }
    server {
      listenHost
      listenPort
      maxConnections
    }
    security {
      mode
    }
    logging {
      level
      format
    }
    # ... other sections
  }
}

# Get individual sections
query {
  instanceConfig {
    name
    instanceId
  }

  serverConfig {
    listenHost
    listenPort
    maxConnections
  }

  loggingConfig {
    level
    format
    auditEnabled
  }
}

# Get version info
query {
  versionInfo {
    binaryVersion
    layoutVersion
    dataFormatVersion
    walFormatVersion
    protocolVersion
  }
}

# Get instance metadata
query {
  instanceMetadata {
    layoutVersion
    instanceId
    createdAt
    dataFormatVersion
  }
}
```

### Configuration Mutations

```graphql
# Reload configuration
mutation {
  reloadConfig {
    success
    message
    reloadedAt
  }
}

# Update logging level
mutation {
  updateLoggingLevel(level: "debug") {
    level
    format
  }
}

# Update cache settings
mutation {
  updateCacheSettings(input: {
    enabled: true
    maxSizeMb: 1024
    queryCacheEnabled: true
  }) {
    enabled
    maxSizeMb
    mlEnabled
    queryCacheEnabled
  }
}

# Set compatibility mode
mutation {
  setCompatMode(strict: true) {
    failOnUnsupportedLayout
    failOnUnsupportedDataFormat
  }
}
```

---

## Configuration File Reference

### Full rustydb.toml Format

See `conf/rustydb.toml` for the complete canonical template with all options documented.

### Key Sections

| Section | Description |
|---------|-------------|
| `[instance]` | Instance name and identity |
| `[paths]` | Directory paths (relative to --home) |
| `[server]` | Server bind address, ports, limits |
| `[server.ipc]` | Unix sockets / Windows named pipes |
| `[security]` | Security mode (dev/prod) |
| `[tls]` | TLS/SSL configuration |
| `[auth]` | Authentication settings |
| `[logging]` | Log output, format, rotation |
| `[storage]` | Storage engine settings |
| `[wal]` | Write-ahead log settings |
| `[cache]` | Cache configuration |
| `[metrics]` | Prometheus metrics |
| `[diagnostics]` | Diagnostic bundle settings |
| `[compat]` | Version compatibility settings |

### Override Mechanism

Files in `conf/overrides.d/*.toml` are loaded in lexicographic order and merged:

```
conf/
├── rustydb.toml           # Base (loaded first)
└── overrides.d/
    ├── 10-logging.toml    # Loaded second
    ├── 20-cache.toml      # Loaded third
    └── 99-local.toml      # Loaded last (highest priority)
```

---

## Instance Metadata

### Metadata Files

| File | Content |
|------|---------|
| `data/meta/layout-version` | "1.0" |
| `data/meta/instance-id` | UUID v4 |
| `data/meta/created-at` | RFC3339 timestamp |
| `data/meta/data-format-version` | Integer (e.g., 2) |
| `data/meta/wal-format-version` | Integer (e.g., 2) |
| `data/meta/protocol-version` | Integer (e.g., 2) |
| `data/meta/compat.json` | Compatibility hints |

### Version Compatibility

RustyDB 0.3.001 supports:
- Layout version: 1.0
- Data format: 1-2 (RW), 1 (RO for older)
- WAL format: 1-2
- Protocol: 1-2

---

## Service Deployment

### Linux (systemd)

```bash
# Multi-instance
sudo systemctl enable --now rustydb@prod

# Commands
sudo systemctl start rustydb@prod
sudo systemctl stop rustydb@prod
sudo journalctl -u rustydb@prod -f
```

### Windows

```batch
# Install
install-service.bat prod

# Commands
sc start RustyDB_prod
sc stop RustyDB_prod
```

---

## Code Examples

### Rust: Reading Configuration

```rust
use rusty_db::metadata::{InstanceMetadata, MetaPaths};
use std::path::Path;

fn main() -> rusty_db::Result<()> {
    let home = Path::new("/var/lib/rustydb/instances/prod");
    let paths = MetaPaths::from_instance_root(home);

    let metadata = InstanceMetadata::load(&paths.meta_dir)?;
    println!("Instance: {}", metadata.instance_id);
    println!("Data Format: {}", metadata.data_format);

    Ok(())
}
```

### Python: REST API Usage

```python
import requests

base_url = "http://localhost:54321/api/v1"

# Get full config
response = requests.get(f"{base_url}/config")
config = response.json()
print(f"Server port: {config['server']['listen_port']}")

# Reload config
response = requests.put(f"{base_url}/config/reload")
result = response.json()
print(f"Reload: {result['message']}")

# Get version
response = requests.get(f"{base_url}/metadata/version")
version = response.json()
print(f"RustyDB {version['binary_version']}")
```

### JavaScript: GraphQL Queries

```javascript
const query = `
  query {
    config {
      instance { name }
      server { listenPort }
    }
    versionInfo {
      binaryVersion
    }
  }
`;

fetch('http://localhost:54321/graphql', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ query })
})
.then(res => res.json())
.then(data => {
  console.log('Instance:', data.data.config.instance.name);
  console.log('Port:', data.data.config.server.listenPort);
  console.log('Version:', data.data.versionInfo.binaryVersion);
});
```

---

## Migration Guide

### From Previous Versions

1. **Backup existing data**
2. **Update binary** to 0.3.001
3. **Run upgrade**: `rusty-db-cli upgrade --home <path>`
4. **Verify metadata**: Check `data/meta/` files
5. **Start server**: `rusty-db-server --home <path>`

### Breaking Changes

None in 0.3.001 - this is the initial release of the configuration system.

---

## Appendix: API Endpoints Summary

### REST Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /api/v1/config | Get full configuration |
| GET | /api/v1/config/instance | Get instance config |
| GET | /api/v1/config/paths | Get paths config |
| GET | /api/v1/config/server | Get server config |
| GET | /api/v1/config/security | Get security config |
| GET | /api/v1/config/logging | Get logging config |
| GET | /api/v1/config/storage | Get storage config |
| GET | /api/v1/config/wal | Get WAL config |
| GET | /api/v1/config/cache | Get cache config |
| GET | /api/v1/config/metrics | Get metrics config |
| GET | /api/v1/config/diagnostics | Get diagnostics config |
| GET | /api/v1/config/compat | Get compatibility config |
| GET | /api/v1/config/resolved | Get merged config |
| PUT | /api/v1/config/reload | Reload configuration |
| GET | /api/v1/metadata | Get instance metadata |
| GET | /api/v1/metadata/version | Get version info |

### GraphQL Operations

| Type | Name | Description |
|------|------|-------------|
| Query | config | Full configuration |
| Query | instanceConfig | Instance configuration |
| Query | serverConfig | Server configuration |
| Query | loggingConfig | Logging configuration |
| Query | storageConfig | Storage configuration |
| Query | walConfig | WAL configuration |
| Query | cacheConfig | Cache configuration |
| Query | metricsConfig | Metrics configuration |
| Query | diagnosticsConfig | Diagnostics configuration |
| Query | compatConfig | Compatibility configuration |
| Query | resolvedConfig | Merged configuration |
| Query | versionInfo | Version information |
| Query | instanceMetadata | Instance metadata |
| Mutation | reloadConfig | Reload from disk |
| Mutation | updateLoggingLevel | Change log level |
| Mutation | updateCacheSettings | Update cache config |
| Mutation | setCompatMode | Set compatibility strictness |
