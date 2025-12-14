/**
 * RustyDB Node.js Adapter
 * 
 * Production-ready TypeScript adapter for RustyDB - Enterprise-grade database
 * with Oracle compatibility.
 * 
 * @module @rustydb/adapter
 * @version 0.2.640
 * 
 * @example
 * ```typescript
 * import { createRustyDbClient, createConfig } from '@rustydb/adapter';
 * 
 * const config = createConfig()
 *   .server({ host: 'localhost', port: 5432 })
 *   .api({ baseUrl: 'http://localhost:8080' })
 *   .autoStart(true)
 *   .autoStop(true)
 *   .build();
 * 
 * const client = await createRustyDbClient(config);
 * await client.initialize();
 * 
 * // Use the client...
 * 
 * await client.shutdown();
 * ```
 */

// ============================================================================
// Core Client
// ============================================================================

export {
  RustyDbClient,
  ServerProcessManager,
  HttpClient,
  WebSocketClient,
  ClientEvents,
} from './client';

// ============================================================================
// Configuration
// ============================================================================

export {
  RustyDbConfig,
  ConfigBuilder,
  createConfig,
  mergeConfigs,
  getServerConfig,
  getApiConfig,
  getWebSocketConfig,
  validateConfig,
  loadConfigFromEnv,
  DEFAULT_SERVER_CONFIG,
  DEFAULT_API_CONFIG,
  DEFAULT_GRAPHQL_CONFIG,
  DEFAULT_WS_CONFIG,
  DEFAULT_BINARY_PATHS,
} from './config';

// ============================================================================
// Type Definitions
// ============================================================================

export {
  // Core types
  TransactionId,
  SessionId,
  PageId,
  TableId,
  IndexId,
  Timestamp,
  
  // Enums
  IsolationLevel,
  TransactionState,
  LockMode,
  HealthStatus,
  ErrorCode,
  LogLevel,
  
  // Interfaces
  ServiceStatus,
  ApiError,
  ServerConfig,
  BaseClientConfig,
  WebSocketConfig,
  PaginationParams,
  PaginatedResponse,
  QueryOptions,
  QueryResult,
  QueryResultMetadata,
  ColumnMetadata,
  Metric,
  MetricValue,
  TimeSeriesPoint,
  PerformanceStats,
  ResourceLimits,
  ResourceUsage,
  
  // Utility types
  Callback,
  EventHandler,
  EventMap,
  DeepPartial,
  DeepRequired,
  UnwrapPromise,
  JsonValue,
  JsonObject,
} from './types';

// ============================================================================
// Utilities
// ============================================================================

export {
  // Error handling
  createApiError,
  isApiError,
  getErrorMessage,
  withErrorHandling,
  
  // Async utilities
  sleep,
  retry,
  withTimeout,
  createDeferred,
  Deferred,
  
  // Validation
  isValidUuid,
  isNonEmptyString,
  isPositiveNumber,
  isNonNegativeNumber,
  validateRequired,
  
  // Data transformation
  snakeToCamel,
  camelToSnake,
  deepClone,
  omit,
  pick,
  
  // Time utilities
  now,
  formatTimestamp,
  parseTimestamp,
  duration,
  formatDuration,
  
  // URL utilities
  buildUrl,
  parseQueryParams,
  
  // Collection utilities
  groupBy,
  keyBy,
  chunk,
  unique,
  flatten,
  
  // String utilities
  truncate,
  capitalize,
  randomString,
  
  // Logging
  Logger,
  createLogger,
} from './utils';

// ============================================================================
// API Clients
// ============================================================================

export { StorageClient, createStorageClient } from './api/storage';
export { TransactionClient, createTransactionClient } from './api/transactions';
export { SecurityClient, createSecurityClient } from './api/security';
export { QueryOptimizerClient, createQueryOptimizerClient } from './api/query-optimizer';
export { MonitoringClient, createMonitoringClient } from './api/monitoring';
export { NetworkPoolClient, createNetworkPoolClient } from './api/network-pool';
export { ReplicationRACClient, createReplicationRACClient } from './api/replication-rac';
export { BackupRecoveryClient, createBackupRecoveryClient } from './api/backup-recovery';
export { MLAnalyticsClient, createMLAnalyticsClient } from './api/ml-analytics';
export { GraphQLClient, createGraphQLClient } from './api/graphql-client';
export { IndexMemoryClient, createIndexMemoryClient } from './api/index-memory';
export { EnterpriseSpatialClient, createEnterpriseSpatialClient } from './api/enterprise-spatial';

// Re-export all types from API modules
export * from './api/storage';
export * from './api/transactions';
export * from './api/security';
export * from './api/query-optimizer';
export * from './api/monitoring';
export * from './api/network-pool';
export * from './api/replication-rac';
export * from './api/backup-recovery';
export * from './api/ml-analytics';
export * from './api/graphql-client';
export * from './api/index-memory';
export * from './api/enterprise-spatial';

// ============================================================================
// Factory Functions
// ============================================================================

import { RustyDbClient } from './client';
import { RustyDbConfig } from './config';

/**
 * Create a new RustyDB client instance
 * 
 * @param config - Client configuration
 * @returns Configured RustyDB client instance
 * 
 * @example
 * ```typescript
 * const client = createRustyDbClient({
 *   server: { host: 'localhost', port: 5432 },
 *   api: { baseUrl: 'http://localhost:8080' },
 *   autoStart: true,
 * });
 * 
 * await client.initialize();
 * ```
 */
export function createRustyDbClient(config?: RustyDbConfig): RustyDbClient {
  return new RustyDbClient(config);
}

/**
 * Create and initialize a RustyDB client in one step
 * 
 * @param config - Client configuration
 * @returns Initialized RustyDB client instance
 * 
 * @example
 * ```typescript
 * const client = await initializeRustyDbClient({
 *   autoStart: true,
 *   autoStop: true,
 * });
 * ```
 */
export async function initializeRustyDbClient(config?: RustyDbConfig): Promise<RustyDbClient> {
  const client = new RustyDbClient(config);
  await client.initialize();
  return client;
}

// ============================================================================
// Default Export
// ============================================================================

export default {
  // Client
  createRustyDbClient,
  initializeRustyDbClient,
  RustyDbClient,
  
  // Configuration
  createConfig,
  mergeConfigs,
  validateConfig,
  loadConfigFromEnv,
  
  // Utilities
  createLogger,
  sleep,
  retry,
  withTimeout,
};
