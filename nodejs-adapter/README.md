# RustyDB Node.js Adapter

[![Version](https://img.shields.io/badge/version-0.2.640-blue.svg)](https://github.com/harborgrid-justin/rusty-db)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.3-blue.svg)](https://www.typescriptlang.org/)

Production-ready TypeScript/JavaScript adapter for RustyDB - Enterprise-grade database with Oracle compatibility.

## Features

- **Binary Process Management**: Automatic spawning and lifecycle management of RustyDB server
- **REST API Client**: Full coverage of all RustyDB REST endpoints
- **GraphQL Support**: Complete GraphQL query, mutation, and subscription support
- **WebSocket Communication**: Real-time data streaming with auto-reconnect
- **Type Safety**: Full TypeScript definitions for all APIs
- **Configuration Builder**: Fluent API for easy configuration
- **Event System**: Event-driven architecture for process and connection events
- **Cross-Platform**: Works on Windows, Linux, and macOS

## Installation

```bash
cd nodejs-adapter
npm install
npm run build
```

## Quick Start

```typescript
import { createRustyDbClient, createConfig } from '@rustydb/adapter';

// Create configuration
const config = createConfig()
  .server({ host: 'localhost', port: 5432 })
  .api({ baseUrl: 'http://localhost:8080' })
  .autoStart(true)  // Automatically start the server
  .autoStop(true)   // Automatically stop on shutdown
  .build();

// Initialize client
const client = await createRustyDbClient(config);
await client.initialize();

// Use the client...
const http = client.getHttpClient();
const health = await http.healthCheck();
console.log('Server status:', health.status);

// Cleanup
await client.shutdown();
```

## API Modules

The adapter includes 10 specialized API client modules:

1. **Storage API** (`storage.ts`) - Storage, buffer pool, tablespaces, partitions
2. **Transaction API** (`transactions.ts`) - Transactions, MVCC, locks, savepoints
3. **Security API** (`security.ts`) - TDE, data masking, RBAC, audit logging
4. **Query/Optimizer API** (`query-optimizer.ts`) - Query execution, explain plans, hints
5. **Monitoring API** (`monitoring.ts`) - Health checks, metrics, performance stats
6. **Network/Pool API** (`network-pool.ts`) - Connection management, cluster topology
7. **Replication/RAC API** (`replication-rac.ts`) - Replication, RAC, Cache Fusion
8. **Backup/Recovery API** (`backup-recovery.ts`) - Backups, restores, PITR
9. **ML/Analytics API** (`ml-analytics.ts`) - Machine learning, OLAP, analytics
10. **GraphQL Client** (`graphql-client.ts`) - GraphQL queries, mutations, subscriptions

## Examples

See `examples/basic-usage.ts` for comprehensive usage examples including:
- Basic client setup
- Storage management
- Transaction handling
- Security features
- Monitoring and health checks
- Event handling
- Complete application example

## Documentation

- **Master Report**: `.scratchpad/NODEJS_ADAPTER_MASTER_REPORT.md` - Complete architecture and coordination
- **API Documentation**: Each API module includes inline JSDoc documentation
- **Type Definitions**: `src/types/index.ts` - All TypeScript type definitions

## Configuration

### Using Environment Variables

```bash
export RUSTYDB_HOST=localhost
export RUSTYDB_PORT=5432
export RUSTYDB_API_URL=http://localhost:8080
export RUSTYDB_AUTO_START=true
```

```typescript
import { loadConfigFromEnv } from '@rustydb/adapter';

const config = loadConfigFromEnv();
const client = await createRustyDbClient(config);
```

### Using Configuration Builder

```typescript
import { createConfig } from '@rustydb/adapter';

const config = createConfig()
  .server({ 
    host: 'localhost', 
    port: 5432,
    dataDir: './data',
    logLevel: 'info',
    maxConnections: 100 
  })
  .api({ 
    baseUrl: 'http://localhost:8080',
    timeout: 30000 
  })
  .websocket({ 
    url: 'ws://localhost:8080/ws',
    reconnect: true 
  })
  .binaries({
    server: 'target/release/rusty-db-server',
    cli: 'target/release/rusty-db-cli'
  })
  .autoStart(true)
  .autoStop(true)
  .build();
```

## Development

### Build

```bash
npm run build         # Build TypeScript
npm run build:watch   # Build and watch for changes
```

### Testing

```bash
npm test              # Run all tests
npm run test:watch    # Run tests in watch mode
npm run test:coverage # Run tests with coverage
```

### Linting and Formatting

```bash
npm run lint          # Run ESLint
npm run lint:fix      # Fix ESLint issues
npm run format        # Format with Prettier
npm run format:check  # Check formatting
```

## Architecture

```
Core Infrastructure:
├── Client Management (client.ts)
│   ├── ServerProcessManager - Binary lifecycle
│   ├── HttpClient - REST API
│   ├── WebSocketClient - Real-time communication
│   └── RustyDbClient - Main orchestrator
├── Configuration (config/)
│   ├── Builder pattern
│   ├── Validation
│   └── Environment loading
├── Types (types/)
│   ├── Common types
│   └── GraphQL types
└── Utilities (utils/)
    ├── Error handling
    ├── Async utilities
    ├── Validation
    └── Data transformation

API Clients (api/):
├── storage.ts
├── transactions.ts
├── security.ts
├── query-optimizer.ts
├── monitoring.ts
├── network-pool.ts
├── replication-rac.ts
├── backup-recovery.ts
├── ml-analytics.ts
└── graphql-client.ts
```

## Binary Information

- **Server Binary**: `target/release/rusty-db-server`
- **CLI Binary**: `target/release/rusty-db-cli`
- **Default Port**: 5432 (configurable)
- **REST API**: http://localhost:8080/api/v1/
- **GraphQL**: http://localhost:8080/graphql
- **WebSocket**: ws://localhost:8080/ws

## Contributing

This adapter was developed by the PhD Engineering Team (Agents 1-11) as part of the RustyDB project.

See `.scratchpad/NODEJS_ADAPTER_COORDINATION_2025_12_13.md` for agent assignments and coordination.

## License

MIT

## Version

Current version: 0.2.640 (aligned with RustyDB server version)
