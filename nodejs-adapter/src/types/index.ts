/**
 * Common type definitions shared across all RustyDB adapter modules
 * @module types
 */

// ============================================================================
// Core Database Types
// ============================================================================

/**
 * Transaction ID (UUID format)
 */
export type TransactionId = string;

/**
 * Session ID (UUID format)
 */
export type SessionId = string;

/**
 * Page ID
 */
export type PageId = number;

/**
 * Table ID
 */
export type TableId = string;

/**
 * Index ID
 */
export type IndexId = string;

/**
 * Timestamp (Unix milliseconds)
 */
export type Timestamp = number;

// ============================================================================
// Transaction Types
// ============================================================================

/**
 * Transaction isolation levels
 */
export enum IsolationLevel {
  READ_UNCOMMITTED = 'READ_UNCOMMITTED',
  READ_COMMITTED = 'READ_COMMITTED',
  REPEATABLE_READ = 'REPEATABLE_READ',
  SERIALIZABLE = 'SERIALIZABLE',
  SNAPSHOT_ISOLATION = 'SNAPSHOT_ISOLATION',
}

/**
 * Transaction state
 */
export enum TransactionState {
  ACTIVE = 'active',
  PREPARING = 'preparing',
  PREPARED = 'prepared',
  COMMITTING = 'committing',
  COMMITTED = 'committed',
  ABORTING = 'aborting',
  ABORTED = 'aborted',
}

/**
 * Lock modes
 */
export enum LockMode {
  SHARED = 'shared',
  EXCLUSIVE = 'exclusive',
  UPDATE = 'update',
  INTENTION_SHARED = 'intention_shared',
  INTENTION_EXCLUSIVE = 'intention_exclusive',
}

// ============================================================================
// Health & Status Types
// ============================================================================

/**
 * Component health status
 */
export enum HealthStatus {
  HEALTHY = 'healthy',
  DEGRADED = 'degraded',
  UNHEALTHY = 'unhealthy',
  UNKNOWN = 'unknown',
}

/**
 * Service status
 */
export interface ServiceStatus {
  name: string;
  status: HealthStatus;
  uptime_seconds: number;
  message?: string;
  last_check: Timestamp;
}

// ============================================================================
// Error Types
// ============================================================================

/**
 * Standard API error response
 */
export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  stack?: string;
}

/**
 * Error codes
 */
export enum ErrorCode {
  // General errors
  UNKNOWN_ERROR = 'UNKNOWN_ERROR',
  INTERNAL_ERROR = 'INTERNAL_ERROR',
  INVALID_REQUEST = 'INVALID_REQUEST',
  INVALID_PARAMETER = 'INVALID_PARAMETER',
  
  // Transaction errors
  TRANSACTION_NOT_FOUND = 'TRANSACTION_NOT_FOUND',
  TRANSACTION_ABORTED = 'TRANSACTION_ABORTED',
  DEADLOCK_DETECTED = 'DEADLOCK_DETECTED',
  SERIALIZATION_FAILURE = 'SERIALIZATION_FAILURE',
  
  // Storage errors
  PAGE_NOT_FOUND = 'PAGE_NOT_FOUND',
  DISK_FULL = 'DISK_FULL',
  IO_ERROR = 'IO_ERROR',
  
  // Security errors
  AUTHENTICATION_FAILED = 'AUTHENTICATION_FAILED',
  AUTHORIZATION_FAILED = 'AUTHORIZATION_FAILED',
  PERMISSION_DENIED = 'PERMISSION_DENIED',
  
  // Network errors
  CONNECTION_FAILED = 'CONNECTION_FAILED',
  TIMEOUT = 'TIMEOUT',
  CONNECTION_CLOSED = 'CONNECTION_CLOSED',
}

// ============================================================================
// Configuration Types
// ============================================================================

/**
 * Server configuration
 */
export interface ServerConfig {
  host: string;
  port: number;
  dataDir?: string;
  logLevel?: LogLevel;
  maxConnections?: number;
}

/**
 * Log levels
 */
export enum LogLevel {
  TRACE = 'trace',
  DEBUG = 'debug',
  INFO = 'info',
  WARN = 'warn',
  ERROR = 'error',
}

// ============================================================================
// Client Configuration Types
// ============================================================================

/**
 * Base client configuration for all API clients
 */
export interface BaseClientConfig {
  baseUrl: string;
  apiVersion?: string;
  timeout?: number;
  headers?: Record<string, string>;
}

/**
 * WebSocket configuration
 */
export interface WebSocketConfig {
  url: string;
  reconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  pingInterval?: number;
  headers?: Record<string, string>;
}

// ============================================================================
// Pagination Types
// ============================================================================

/**
 * Pagination request parameters
 */
export interface PaginationParams {
  page?: number;
  limit?: number;
  offset?: number;
}

/**
 * Paginated response wrapper
 */
export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  limit: number;
  has_more: boolean;
}

// ============================================================================
// Query Types
// ============================================================================

/**
 * SQL query execution options
 */
export interface QueryOptions {
  timeout?: number;
  maxRows?: number;
  fetchSize?: number;
  prepareStatement?: boolean;
}

/**
 * Query result metadata
 */
export interface QueryResultMetadata {
  column_count: number;
  row_count: number;
  execution_time_ms: number;
  columns: ColumnMetadata[];
}

/**
 * Column metadata
 */
export interface ColumnMetadata {
  name: string;
  type: string;
  nullable: boolean;
  precision?: number;
  scale?: number;
}

/**
 * Query execution result
 */
export interface QueryResult<T = unknown> {
  rows: T[];
  metadata: QueryResultMetadata;
  warnings?: string[];
}

// ============================================================================
// Monitoring & Metrics Types
// ============================================================================

/**
 * Metric value types
 */
export type MetricValue = number | string | boolean;

/**
 * Metric data point
 */
export interface Metric {
  name: string;
  value: MetricValue;
  timestamp: Timestamp;
  labels?: Record<string, string>;
}

/**
 * Time series data point
 */
export interface TimeSeriesPoint {
  timestamp: Timestamp;
  value: number;
}

/**
 * Performance statistics
 */
export interface PerformanceStats {
  cpu_usage_percent: number;
  memory_usage_bytes: number;
  memory_usage_percent: number;
  disk_usage_bytes: number;
  disk_usage_percent: number;
  network_rx_bytes: number;
  network_tx_bytes: number;
}

// ============================================================================
// Resource Types
// ============================================================================

/**
 * Resource limits
 */
export interface ResourceLimits {
  max_memory_bytes?: number;
  max_cpu_percent?: number;
  max_connections?: number;
  max_query_time_ms?: number;
}

/**
 * Resource usage
 */
export interface ResourceUsage {
  memory_bytes: number;
  cpu_percent: number;
  connections: number;
  queries_per_second: number;
}

// ============================================================================
// Callback & Event Types
// ============================================================================

/**
 * Generic callback function
 */
export type Callback<T = void> = (error: Error | null, result?: T) => void;

/**
 * Event handler function
 */
export type EventHandler<T = unknown> = (data: T) => void | Promise<void>;

/**
 * Event map for type-safe event emitters
 */
export interface EventMap {
  [event: string]: unknown;
}

// ============================================================================
// Utility Types
// ============================================================================

/**
 * Make all properties optional recursively
 */
export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

/**
 * Make all properties required recursively
 */
export type DeepRequired<T> = {
  [P in keyof T]-?: T[P] extends object ? DeepRequired<T[P]> : T[P];
};

/**
 * Extract promise type
 */
export type UnwrapPromise<T> = T extends Promise<infer U> ? U : T;

/**
 * JSON-serializable types
 */
export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

/**
 * JSON object
 */
export type JsonObject = { [key: string]: JsonValue };

// ============================================================================
// Re-export GraphQL types
// ============================================================================

export * from './graphql-types';
