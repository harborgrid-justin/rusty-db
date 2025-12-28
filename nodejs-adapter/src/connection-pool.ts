/**
 * Advanced Connection Pool for RustyDB
 *
 * Provides efficient connection pooling with lifecycle management,
 * health checks, and automatic reconnection.
 *
 * @module connection-pool
 */

import { EventEmitter } from 'eventemitter3';
import { BaseClient } from './api/base-client';

/**
 * Connection pool configuration
 */
export interface ConnectionPoolConfig {
  /** Minimum number of connections to maintain */
  minConnections?: number;
  /** Maximum number of connections allowed */
  maxConnections?: number;
  /** Connection acquisition timeout in milliseconds */
  acquireTimeout?: number;
  /** Idle connection timeout in milliseconds */
  idleTimeout?: number;
  /** Connection validation query */
  validationQuery?: string;
  /** Validate connections on acquire */
  validateOnAcquire?: boolean;
  /** Validate connections on return */
  validateOnReturn?: boolean;
  /** Health check interval in milliseconds */
  healthCheckInterval?: number;
  /** Enable connection lifecycle logging */
  logging?: boolean;
}

/**
 * Connection state
 */
export enum ConnectionState {
  IDLE = 'idle',
  ACTIVE = 'active',
  VALIDATING = 'validating',
  CLOSED = 'closed',
  ERROR = 'error',
}

/**
 * Pooled connection
 */
export interface PooledConnection {
  id: string;
  client: BaseClient;
  state: ConnectionState;
  createdAt: Date;
  lastUsedAt: Date;
  usageCount: number;
  errors: number;
}

/**
 * Connection pool statistics
 */
export interface PoolStats {
  totalConnections: number;
  activeConnections: number;
  idleConnections: number;
  waitingRequests: number;
  totalAcquired: number;
  totalReleased: number;
  totalCreated: number;
  totalDestroyed: number;
  totalErrors: number;
  avgAcquireTimeMs: number;
  avgConnectionLifetimeMs: number;
}

/**
 * Pool events
 */
export interface PoolEvents {
  acquire: (connection: PooledConnection) => void;
  release: (connection: PooledConnection) => void;
  create: (connection: PooledConnection) => void;
  destroy: (connection: PooledConnection) => void;
  error: (error: Error, connection?: PooledConnection) => void;
  drain: () => void;
}

/**
 * Connection factory function
 */
export type ConnectionFactory = () => Promise<BaseClient>;

/**
 * Advanced connection pool
 *
 * Manages a pool of database connections with automatic lifecycle management,
 * health checks, and connection validation.
 */
export class ConnectionPool extends EventEmitter<PoolEvents> {
  private config: Required<ConnectionPoolConfig>;
  private factory: ConnectionFactory;
  private connections: Map<string, PooledConnection> = new Map();
  private idleConnections: PooledConnection[] = [];
  private waitingQueue: Array<{
    resolve: (connection: PooledConnection) => void;
    reject: (error: Error) => void;
    timestamp: number;
  }> = [];
  private stats: PoolStats = {
    totalConnections: 0,
    activeConnections: 0,
    idleConnections: 0,
    waitingRequests: 0,
    totalAcquired: 0,
    totalReleased: 0,
    totalCreated: 0,
    totalDestroyed: 0,
    totalErrors: 0,
    avgAcquireTimeMs: 0,
    avgConnectionLifetimeMs: 0,
  };
  private healthCheckTimer: NodeJS.Timeout | null = null;
  private closed: boolean = false;

  constructor(factory: ConnectionFactory, config: ConnectionPoolConfig = {}) {
    super();

    this.factory = factory;
    this.config = {
      minConnections: config.minConnections || 2,
      maxConnections: config.maxConnections || 10,
      acquireTimeout: config.acquireTimeout || 30000,
      idleTimeout: config.idleTimeout || 300000, // 5 minutes
      validationQuery: config.validationQuery || 'SELECT 1',
      validateOnAcquire: config.validateOnAcquire ?? true,
      validateOnReturn: config.validateOnReturn ?? false,
      healthCheckInterval: config.healthCheckInterval || 60000, // 1 minute
      logging: config.logging ?? false,
    };
  }

  /**
   * Initialize the connection pool
   *
   * @example
   * ```typescript
   * const pool = new ConnectionPool(() => createClient(), {
   *   minConnections: 5,
   *   maxConnections: 20
   * });
   * await pool.initialize();
   * ```
   */
  async initialize(): Promise<void> {
    if (this.closed) {
      throw new Error('Cannot initialize closed pool');
    }

    this.log('Initializing connection pool...');

    // Create minimum connections
    const promises: Promise<void>[] = [];
    for (let i = 0; i < this.config.minConnections; i++) {
      promises.push(this.createConnection());
    }

    await Promise.all(promises);

    // Start health check timer
    this.startHealthCheck();

    this.log(`Pool initialized with ${this.connections.size} connections`);
  }

  /**
   * Acquire a connection from the pool
   *
   * @returns Pooled connection
   * @throws Error if timeout is reached or pool is closed
   *
   * @example
   * ```typescript
   * const connection = await pool.acquire();
   * try {
   *   // Use connection
   *   await connection.client.get('/api/v1/health');
   * } finally {
   *   await pool.release(connection);
   * }
   * ```
   */
  async acquire(): Promise<PooledConnection> {
    if (this.closed) {
      throw new Error('Cannot acquire from closed pool');
    }

    const startTime = Date.now();

    // Try to get idle connection
    const connection = await this.getOrCreateConnection();

    // Validate if configured
    if (this.config.validateOnAcquire) {
      const isValid = await this.validateConnection(connection);
      if (!isValid) {
        await this.destroyConnection(connection);
        return this.acquire(); // Retry
      }
    }

    // Update connection state
    connection.state = ConnectionState.ACTIVE;
    connection.lastUsedAt = new Date();
    connection.usageCount++;

    // Update stats
    this.stats.totalAcquired++;
    this.stats.activeConnections++;
    this.stats.idleConnections = this.idleConnections.length;
    const acquireTime = Date.now() - startTime;
    this.stats.avgAcquireTimeMs =
      (this.stats.avgAcquireTimeMs * (this.stats.totalAcquired - 1) + acquireTime) /
      this.stats.totalAcquired;

    this.emit('acquire', connection);
    this.log(`Acquired connection ${connection.id}`);

    return connection;
  }

  /**
   * Release a connection back to the pool
   *
   * @param connection - Connection to release
   */
  async release(connection: PooledConnection): Promise<void> {
    if (!this.connections.has(connection.id)) {
      this.log(`Attempted to release unknown connection ${connection.id}`);
      return;
    }

    // Validate if configured
    if (this.config.validateOnReturn) {
      const isValid = await this.validateConnection(connection);
      if (!isValid) {
        await this.destroyConnection(connection);
        return;
      }
    }

    // Update connection state
    connection.state = ConnectionState.IDLE;
    connection.lastUsedAt = new Date();

    // Check if there are waiting requests
    if (this.waitingQueue.length > 0) {
      const waiter = this.waitingQueue.shift()!;
      waiter.resolve(connection);
      return;
    }

    // Return to idle pool
    this.idleConnections.push(connection);

    // Update stats
    this.stats.totalReleased++;
    this.stats.activeConnections--;
    this.stats.idleConnections = this.idleConnections.length;

    this.emit('release', connection);
    this.log(`Released connection ${connection.id}`);
  }

  /**
   * Get pool statistics
   */
  getStats(): PoolStats {
    return { ...this.stats };
  }

  /**
   * Get all connections (for debugging)
   */
  getConnections(): PooledConnection[] {
    return Array.from(this.connections.values());
  }

  /**
   * Close the pool and all connections
   */
  async close(): Promise<void> {
    if (this.closed) {
      return;
    }

    this.log('Closing connection pool...');
    this.closed = true;

    // Stop health check
    this.stopHealthCheck();

    // Reject all waiting requests
    for (const waiter of this.waitingQueue) {
      waiter.reject(new Error('Pool is closing'));
    }
    this.waitingQueue = [];

    // Close all connections
    const promises: Promise<void>[] = [];
    for (const connection of this.connections.values()) {
      promises.push(this.destroyConnection(connection));
    }

    await Promise.all(promises);

    this.log('Pool closed');
    this.emit('drain');
  }

  /**
   * Execute a function with an acquired connection (auto-release)
   *
   * @param fn - Function to execute with connection
   * @returns Result of the function
   *
   * @example
   * ```typescript
   * const result = await pool.withConnection(async (connection) => {
   *   return connection.client.get('/api/v1/users');
   * });
   * ```
   */
  async withConnection<T>(
    fn: (connection: PooledConnection) => Promise<T>
  ): Promise<T> {
    const connection = await this.acquire();
    try {
      return await fn(connection);
    } finally {
      await this.release(connection);
    }
  }

  /**
   * Get or create a connection
   */
  private async getOrCreateConnection(): Promise<PooledConnection> {
    // Try to get idle connection
    const idleConnection = this.idleConnections.pop();
    if (idleConnection) {
      return idleConnection;
    }

    // Create new connection if under limit
    if (this.connections.size < this.config.maxConnections) {
      await this.createConnection();
      return this.idleConnections.pop()!;
    }

    // Wait for connection to become available
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        const index = this.waitingQueue.findIndex((w) => w.resolve === resolve);
        if (index !== -1) {
          this.waitingQueue.splice(index, 1);
        }
        reject(new Error(`Connection acquire timeout after ${this.config.acquireTimeout}ms`));
      }, this.config.acquireTimeout);

      this.waitingQueue.push({
        resolve: (connection) => {
          clearTimeout(timeout);
          resolve(connection);
        },
        reject: (error) => {
          clearTimeout(timeout);
          reject(error);
        },
        timestamp: Date.now(),
      });

      this.stats.waitingRequests = this.waitingQueue.length;
    });
  }

  /**
   * Create a new connection
   */
  private async createConnection(): Promise<void> {
    try {
      const client = await this.factory();
      const connection: PooledConnection = {
        id: `conn_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        client,
        state: ConnectionState.IDLE,
        createdAt: new Date(),
        lastUsedAt: new Date(),
        usageCount: 0,
        errors: 0,
      };

      this.connections.set(connection.id, connection);
      this.idleConnections.push(connection);

      this.stats.totalCreated++;
      this.stats.totalConnections = this.connections.size;
      this.stats.idleConnections = this.idleConnections.length;

      this.emit('create', connection);
      this.log(`Created connection ${connection.id}`);
    } catch (error) {
      this.stats.totalErrors++;
      this.emit('error', error instanceof Error ? error : new Error(String(error)));
      throw error;
    }
  }

  /**
   * Destroy a connection
   */
  private async destroyConnection(connection: PooledConnection): Promise<void> {
    try {
      // Remove from collections
      this.connections.delete(connection.id);
      const idleIndex = this.idleConnections.indexOf(connection);
      if (idleIndex !== -1) {
        this.idleConnections.splice(idleIndex, 1);
      }

      // Close the client
      if (connection.client && typeof connection.client.close === 'function') {
        await connection.client.close();
      }

      connection.state = ConnectionState.CLOSED;

      this.stats.totalDestroyed++;
      this.stats.totalConnections = this.connections.size;
      this.stats.idleConnections = this.idleConnections.length;

      const lifetime = Date.now() - connection.createdAt.getTime();
      this.stats.avgConnectionLifetimeMs =
        (this.stats.avgConnectionLifetimeMs * (this.stats.totalDestroyed - 1) + lifetime) /
        this.stats.totalDestroyed;

      this.emit('destroy', connection);
      this.log(`Destroyed connection ${connection.id}`);
    } catch (error) {
      this.stats.totalErrors++;
      this.emit('error', error instanceof Error ? error : new Error(String(error)), connection);
    }
  }

  /**
   * Validate a connection
   */
  private async validateConnection(connection: PooledConnection): Promise<boolean> {
    try {
      connection.state = ConnectionState.VALIDATING;

      // Simple ping to verify connection is alive
      await connection.client.get('/api/v1/health');

      return true;
    } catch (error) {
      connection.errors++;
      this.log(`Connection ${connection.id} validation failed:`, error);
      return false;
    }
  }

  /**
   * Start health check timer
   */
  private startHealthCheck(): void {
    this.healthCheckTimer = setInterval(() => {
      this.performHealthCheck();
    }, this.config.healthCheckInterval);
  }

  /**
   * Stop health check timer
   */
  private stopHealthCheck(): void {
    if (this.healthCheckTimer) {
      clearInterval(this.healthCheckTimer);
      this.healthCheckTimer = null;
    }
  }

  /**
   * Perform health check on all connections
   */
  private async performHealthCheck(): Promise<void> {
    this.log('Performing health check...');

    const now = Date.now();
    const destroyPromises: Promise<void>[] = [];

    // Check idle connections for timeout
    for (const connection of this.idleConnections.slice()) {
      const idleTime = now - connection.lastUsedAt.getTime();

      if (idleTime > this.config.idleTimeout) {
        // Don't destroy if we're at minimum connections
        if (this.connections.size > this.config.minConnections) {
          this.log(`Connection ${connection.id} idle timeout, destroying`);
          destroyPromises.push(this.destroyConnection(connection));
        }
      }
    }

    await Promise.all(destroyPromises);

    // Ensure minimum connections
    while (this.connections.size < this.config.minConnections) {
      await this.createConnection();
    }

    this.log(`Health check complete: ${this.connections.size} connections`);
  }

  /**
   * Log a message (if logging is enabled)
   */
  private log(...args: unknown[]): void {
    if (this.config.logging) {
      console.log('[ConnectionPool]', ...args);
    }
  }
}

export default ConnectionPool;
