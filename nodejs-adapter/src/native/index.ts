/**
 * RustyDB Native N-API Bindings
 *
 * Provides high-performance native bindings to RustyDB using N-API.
 * This module offers direct access to the Rust backend for optimal performance.
 *
 * @module native
 */

/**
 * Native binding configuration
 */
export interface NativeBindingConfig {
  /** Path to the native module (.node file) */
  modulePath?: string;
  /** Enable native bindings (falls back to HTTP if false) */
  enabled?: boolean;
  /** Connection pool size for native connections */
  poolSize?: number;
}

/**
 * Native connection handle
 */
export interface NativeConnection {
  id: number;
  isValid: boolean;
}

/**
 * Query execution options for native queries
 */
export interface NativeQueryOptions {
  /** Maximum rows to fetch */
  maxRows?: number;
  /** Timeout in milliseconds */
  timeout?: number;
  /** Use prepared statement */
  prepared?: boolean;
  /** Stream results instead of buffering */
  stream?: boolean;
}

/**
 * Native query result
 */
export interface NativeQueryResult<T = unknown> {
  rows: T[];
  rowCount: number;
  columnCount: number;
  executionTimeMs: number;
  columns: Array<{
    name: string;
    type: string;
    nullable: boolean;
  }>;
}

/**
 * Native prepared statement
 */
export interface NativePreparedStatement {
  id: string;
  sql: string;
  paramCount: number;

  /**
   * Execute the prepared statement with parameters
   */
  execute<T = unknown>(params: unknown[]): Promise<NativeQueryResult<T>>;

  /**
   * Close and deallocate the prepared statement
   */
  close(): Promise<void>;
}

/**
 * RustyDB Native Bindings Interface
 *
 * This interface defines the native methods that will be implemented in Rust
 * using N-API. The actual implementation would be in a separate Rust crate.
 */
export interface RustyDBNativeBindings {
  /**
   * Initialize the native module
   */
  initialize(config: NativeBindingConfig): Promise<void>;

  /**
   * Create a new database connection
   */
  connect(connectionString: string): Promise<NativeConnection>;

  /**
   * Close a database connection
   */
  disconnect(connection: NativeConnection): Promise<void>;

  /**
   * Execute a SQL query
   */
  query<T = unknown>(
    connection: NativeConnection,
    sql: string,
    params?: unknown[],
    options?: NativeQueryOptions
  ): Promise<NativeQueryResult<T>>;

  /**
   * Prepare a SQL statement
   */
  prepare(
    connection: NativeConnection,
    sql: string
  ): Promise<NativePreparedStatement>;

  /**
   * Begin a transaction
   */
  beginTransaction(connection: NativeConnection, isolationLevel?: string): Promise<number>;

  /**
   * Commit a transaction
   */
  commitTransaction(connection: NativeConnection, transactionId: number): Promise<void>;

  /**
   * Rollback a transaction
   */
  rollbackTransaction(connection: NativeConnection, transactionId: number): Promise<void>;

  /**
   * Get connection pool statistics
   */
  getPoolStats(): Promise<{
    totalConnections: number;
    activeConnections: number;
    idleConnections: number;
    waitingRequests: number;
  }>;

  /**
   * Shutdown the native module
   */
  shutdown(): Promise<void>;
}

/**
 * Native bindings wrapper with fallback to HTTP
 */
export class NativeBindingsWrapper {
  private nativeModule: RustyDBNativeBindings | null = null;
  private config: NativeBindingConfig;
  private initialized = false;

  constructor(config: NativeBindingConfig = {}) {
    this.config = {
      enabled: config.enabled ?? true,
      poolSize: config.poolSize ?? 10,
      modulePath: config.modulePath,
    };
  }

  /**
   * Initialize native bindings
   */
  async initialize(): Promise<boolean> {
    if (this.initialized) {
      return this.isNativeAvailable();
    }

    if (!this.config.enabled) {
      console.log('[Native] Native bindings disabled, using HTTP fallback');
      this.initialized = true;
      return false;
    }

    try {
      // Try to load the native module
      // In production, this would be: require('@rustydb/native')
      // For now, we'll mark it as unavailable
      console.log('[Native] Native bindings not yet compiled, using HTTP fallback');
      this.nativeModule = null;
      this.initialized = true;
      return false;
    } catch (error) {
      console.warn('[Native] Failed to load native bindings, using HTTP fallback:', error);
      this.nativeModule = null;
      this.initialized = true;
      return false;
    }
  }

  /**
   * Check if native bindings are available
   */
  isNativeAvailable(): boolean {
    return this.nativeModule !== null;
  }

  /**
   * Get the native module (if available)
   */
  getNativeModule(): RustyDBNativeBindings | null {
    return this.nativeModule;
  }

  /**
   * Shutdown native bindings
   */
  async shutdown(): Promise<void> {
    if (this.nativeModule) {
      await this.nativeModule.shutdown();
      this.nativeModule = null;
    }
    this.initialized = false;
  }
}

/**
 * Global native bindings instance
 */
let globalNativeBindings: NativeBindingsWrapper | null = null;

/**
 * Get or create the global native bindings instance
 */
export function getNativeBindings(config?: NativeBindingConfig): NativeBindingsWrapper {
  if (!globalNativeBindings) {
    globalNativeBindings = new NativeBindingsWrapper(config);
  }
  return globalNativeBindings;
}

/**
 * Initialize native bindings
 */
export async function initializeNativeBindings(config?: NativeBindingConfig): Promise<boolean> {
  const bindings = getNativeBindings(config);
  return bindings.initialize();
}

/**
 * Shutdown native bindings
 */
export async function shutdownNativeBindings(): Promise<void> {
  if (globalNativeBindings) {
    await globalNativeBindings.shutdown();
    globalNativeBindings = null;
  }
}

export default {
  getNativeBindings,
  initializeNativeBindings,
  shutdownNativeBindings,
  NativeBindingsWrapper,
};
