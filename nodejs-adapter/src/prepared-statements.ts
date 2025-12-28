/**
 * Prepared Statement Support for RustyDB
 *
 * Provides prepared statement functionality for improved performance
 * and SQL injection prevention.
 *
 * @module prepared-statements
 */

import { BaseClient } from './api/base-client';
import { QueryResult } from './types';

/**
 * Prepared statement parameter
 */
export type PreparedStatementParam = string | number | boolean | null | Buffer | Date;

/**
 * Prepared statement metadata
 */
export interface PreparedStatementMetadata {
  id: string;
  sql: string;
  paramCount: number;
  paramTypes: string[];
  createdAt: Date;
  executionCount: number;
  avgExecutionTimeMs: number;
}

/**
 * Prepared statement execution options
 */
export interface PreparedStatementExecuteOptions {
  /** Maximum rows to return */
  maxRows?: number;
  /** Timeout in milliseconds */
  timeout?: number;
  /** Fetch size for streaming results */
  fetchSize?: number;
}

/**
 * Prepared Statement class
 *
 * Represents a prepared SQL statement that can be executed multiple times
 * with different parameters for improved performance.
 */
export class PreparedStatement<T = unknown> {
  private client: BaseClient;
  private statementId: string;
  private sql: string;
  private paramCount: number;
  private closed: boolean = false;

  constructor(client: BaseClient, statementId: string, sql: string, paramCount: number) {
    this.client = client;
    this.statementId = statementId;
    this.sql = sql;
    this.paramCount = paramCount;
  }

  /**
   * Get the prepared statement ID
   */
  getId(): string {
    return this.statementId;
  }

  /**
   * Get the SQL query
   */
  getSql(): string {
    return this.sql;
  }

  /**
   * Get the parameter count
   */
  getParamCount(): number {
    return this.paramCount;
  }

  /**
   * Execute the prepared statement with parameters
   *
   * @param params - Query parameters (must match paramCount)
   * @param options - Execution options
   * @returns Query result
   *
   * @example
   * ```typescript
   * const stmt = await client.prepare('SELECT * FROM users WHERE id = $1 AND active = $2');
   * const result = await stmt.execute([123, true]);
   * console.log(`Found ${result.rows.length} users`);
   * ```
   */
  async execute(
    params: PreparedStatementParam[] = [],
    options: PreparedStatementExecuteOptions = {}
  ): Promise<QueryResult<T>> {
    if (this.closed) {
      throw new Error('Cannot execute closed prepared statement');
    }

    if (params.length !== this.paramCount) {
      throw new Error(
        `Parameter count mismatch: expected ${this.paramCount}, got ${params.length}`
      );
    }

    // Convert parameters to appropriate format
    const serializedParams = params.map(this.serializeParam);

    // Execute via REST API
    const result = await this.client['post']<{
      rows: T[];
      metadata: {
        column_count: number;
        row_count: number;
        execution_time_ms: number;
        columns: Array<{
          name: string;
          type: string;
          nullable: boolean;
        }>;
      };
    }>(`/api/v1/prepared-statements/${this.statementId}/execute`, {
      params: serializedParams,
      options: {
        max_rows: options.maxRows,
        timeout: options.timeout,
        fetch_size: options.fetchSize,
      },
    });

    return {
      rows: result.rows,
      metadata: {
        column_count: result.metadata.column_count,
        row_count: result.metadata.row_count,
        execution_time_ms: result.metadata.execution_time_ms,
        columns: result.metadata.columns,
      },
    };
  }

  /**
   * Execute and stream results
   *
   * @param params - Query parameters
   * @param options - Execution options
   * @returns Async iterable of rows
   *
   * @example
   * ```typescript
   * const stmt = await client.prepare('SELECT * FROM large_table WHERE category = $1');
   * for await (const row of stmt.executeStream(['active'])) {
   *   console.log(row);
   * }
   * ```
   */
  async *executeStream(
    params: PreparedStatementParam[] = [],
    options: PreparedStatementExecuteOptions = {}
  ): AsyncIterable<T> {
    if (this.closed) {
      throw new Error('Cannot execute closed prepared statement');
    }

    const fetchSize = options.fetchSize || 100;
    let offset = 0;
    let hasMore = true;

    while (hasMore) {
      const result = await this.execute(params, {
        ...options,
        fetchSize,
      });

      for (const row of result.rows) {
        yield row;
      }

      hasMore = result.rows.length === fetchSize;
      offset += result.rows.length;
    }
  }

  /**
   * Get metadata about the prepared statement
   *
   * @returns Statement metadata
   */
  async getMetadata(): Promise<PreparedStatementMetadata> {
    if (this.closed) {
      throw new Error('Cannot get metadata for closed prepared statement');
    }

    return this.client['get']<PreparedStatementMetadata>(
      `/api/v1/prepared-statements/${this.statementId}/metadata`
    );
  }

  /**
   * Close and deallocate the prepared statement
   *
   * @example
   * ```typescript
   * const stmt = await client.prepare('SELECT * FROM users WHERE id = $1');
   * try {
   *   const result = await stmt.execute([123]);
   *   // ... process result
   * } finally {
   *   await stmt.close();
   * }
   * ```
   */
  async close(): Promise<void> {
    if (this.closed) {
      return;
    }

    await this.client['delete']<void>(`/api/v1/prepared-statements/${this.statementId}`);
    this.closed = true;
  }

  /**
   * Check if the statement is closed
   */
  isClosed(): boolean {
    return this.closed;
  }

  /**
   * Serialize a parameter value
   */
  private serializeParam(param: PreparedStatementParam): unknown {
    if (param === null) {
      return null;
    }

    if (param instanceof Date) {
      return param.toISOString();
    }

    if (param instanceof Buffer) {
      return param.toString('base64');
    }

    return param;
  }
}

/**
 * Prepared statement manager
 *
 * Manages the lifecycle of prepared statements with caching and automatic cleanup.
 */
export class PreparedStatementManager {
  private client: BaseClient;
  private statements: Map<string, PreparedStatement> = new Map();
  private maxCacheSize: number;

  constructor(client: BaseClient, maxCacheSize: number = 100) {
    this.client = client;
    this.maxCacheSize = maxCacheSize;
  }

  /**
   * Prepare a SQL statement
   *
   * @param sql - SQL query with parameter placeholders ($1, $2, etc.)
   * @param cache - Whether to cache the prepared statement (default: true)
   * @returns Prepared statement
   *
   * @example
   * ```typescript
   * const manager = new PreparedStatementManager(client);
   * const stmt = await manager.prepare('SELECT * FROM users WHERE email = $1');
   * const result = await stmt.execute(['user@example.com']);
   * ```
   */
  async prepare<T = unknown>(sql: string, cache: boolean = true): Promise<PreparedStatement<T>> {
    // Check cache first
    if (cache && this.statements.has(sql)) {
      return this.statements.get(sql) as PreparedStatement<T>;
    }

    // Prepare the statement via REST API
    const response = await this.client['post']<{
      statement_id: string;
      sql: string;
      param_count: number;
    }>('/api/v1/prepared-statements', {
      sql,
    });

    const statement = new PreparedStatement<T>(
      this.client,
      response.statement_id,
      response.sql,
      response.param_count
    );

    // Cache if enabled
    if (cache) {
      // Evict oldest statement if cache is full
      if (this.statements.size >= this.maxCacheSize) {
        const firstKey = this.statements.keys().next().value;
        const oldStatement = this.statements.get(firstKey);
        if (oldStatement) {
          await oldStatement.close();
        }
        this.statements.delete(firstKey);
      }

      this.statements.set(sql, statement);
    }

    return statement;
  }

  /**
   * Get a cached prepared statement
   *
   * @param sql - SQL query
   * @returns Prepared statement or null if not cached
   */
  getCached<T = unknown>(sql: string): PreparedStatement<T> | null {
    return (this.statements.get(sql) as PreparedStatement<T>) || null;
  }

  /**
   * Clear the statement cache
   */
  async clearCache(): Promise<void> {
    for (const statement of this.statements.values()) {
      await statement.close();
    }
    this.statements.clear();
  }

  /**
   * Get cache statistics
   */
  getCacheStats(): {
    size: number;
    maxSize: number;
    hitRate: number;
  } {
    return {
      size: this.statements.size,
      maxSize: this.maxCacheSize,
      hitRate: 0, // TODO: Track hits/misses
    };
  }

  /**
   * Close all prepared statements and clear cache
   */
  async close(): Promise<void> {
    await this.clearCache();
  }
}

export default PreparedStatementManager;
