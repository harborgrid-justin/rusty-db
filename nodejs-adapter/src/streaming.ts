/**
 * Streaming Support for RustyDB
 *
 * Provides efficient streaming of large result sets to avoid memory issues
 * when working with millions of rows.
 *
 * @module streaming
 */

import { EventEmitter } from 'eventemitter3';
import { BaseClient } from './api/base-client';

/**
 * Stream options
 */
export interface StreamOptions {
  /** Batch size for fetching rows */
  batchSize?: number;
  /** Maximum number of rows to stream */
  maxRows?: number;
  /** Timeout for each batch fetch */
  timeout?: number;
  /** High water mark for back pressure */
  highWaterMark?: number;
}

/**
 * Stream statistics
 */
export interface StreamStats {
  rowsStreamed: number;
  batchesFetched: number;
  bytesTransferred: number;
  elapsedMs: number;
  avgBatchTimeMs: number;
  rowsPerSecond: number;
}

/**
 * Stream events
 */
export interface StreamEvents<T> {
  data: (row: T) => void;
  batch: (rows: T[], batchNumber: number) => void;
  end: () => void;
  error: (error: Error) => void;
  pause: () => void;
  resume: () => void;
  stats: (stats: StreamStats) => void;
}

/**
 * Query result stream
 *
 * Implements a pull-based streaming interface for large result sets.
 * Supports back pressure and automatic batching.
 */
export class QueryResultStream<T = unknown> extends EventEmitter<StreamEvents<T>> {
  private client: BaseClient;
  private sql: string;
  private params: unknown[];
  private options: Required<StreamOptions>;
  private offset: number = 0;
  private rowsStreamed: number = 0;
  private batchesFetched: number = 0;
  private bytesTransferred: number = 0;
  private startTime: number = 0;
  private paused: boolean = false;
  private ended: boolean = false;
  private destroyed: boolean = false;

  constructor(client: BaseClient, sql: string, params: unknown[] = [], options: StreamOptions = {}) {
    super();

    this.client = client;
    this.sql = sql;
    this.params = params;
    this.options = {
      batchSize: options.batchSize || 100,
      maxRows: options.maxRows || Number.MAX_SAFE_INTEGER,
      timeout: options.timeout || 30000,
      highWaterMark: options.highWaterMark || 1000,
    };
  }

  /**
   * Start streaming
   *
   * @example
   * ```typescript
   * const stream = new QueryResultStream(client, 'SELECT * FROM large_table');
   * stream.on('data', row => console.log(row));
   * stream.on('end', () => console.log('Stream complete'));
   * stream.on('error', err => console.error(err));
   * stream.start();
   * ```
   */
  start(): void {
    if (this.destroyed) {
      throw new Error('Cannot start destroyed stream');
    }

    if (this.startTime === 0) {
      this.startTime = Date.now();
    }

    this.fetchNextBatch();
  }

  /**
   * Pause streaming
   */
  pause(): void {
    if (!this.paused) {
      this.paused = true;
      this.emit('pause');
    }
  }

  /**
   * Resume streaming
   */
  resume(): void {
    if (this.paused) {
      this.paused = false;
      this.emit('resume');
      this.fetchNextBatch();
    }
  }

  /**
   * Destroy the stream and clean up resources
   */
  destroy(): void {
    if (!this.destroyed) {
      this.destroyed = true;
      this.ended = true;
      this.removeAllListeners();
    }
  }

  /**
   * Get current stream statistics
   */
  getStats(): StreamStats {
    const elapsedMs = Date.now() - this.startTime;
    const avgBatchTimeMs = this.batchesFetched > 0 ? elapsedMs / this.batchesFetched : 0;
    const rowsPerSecond = elapsedMs > 0 ? (this.rowsStreamed / elapsedMs) * 1000 : 0;

    return {
      rowsStreamed: this.rowsStreamed,
      batchesFetched: this.batchesFetched,
      bytesTransferred: this.bytesTransferred,
      elapsedMs,
      avgBatchTimeMs,
      rowsPerSecond,
    };
  }

  /**
   * Convert to async iterable
   *
   * @example
   * ```typescript
   * const stream = new QueryResultStream(client, 'SELECT * FROM users');
   * for await (const row of stream) {
   *   console.log(row);
   * }
   * ```
   */
  async *[Symbol.asyncIterator](): AsyncIterableIterator<T> {
    const buffer: T[] = [];
    let resolveNext: ((value: IteratorResult<T>) => void) | null = null;
    let rejectNext: ((error: Error) => void) | null = null;
    let streamEnded = false;

    this.on('data', (row: T) => {
      if (resolveNext) {
        resolveNext({ value: row, done: false });
        resolveNext = null;
        rejectNext = null;
      } else {
        buffer.push(row);
      }
    });

    this.on('error', (error: Error) => {
      if (rejectNext) {
        rejectNext(error);
        resolveNext = null;
        rejectNext = null;
      }
    });

    this.on('end', () => {
      streamEnded = true;
      if (resolveNext) {
        resolveNext({ value: undefined as unknown as T, done: true });
        resolveNext = null;
        rejectNext = null;
      }
    });

    this.start();

    while (true) {
      if (buffer.length > 0) {
        yield buffer.shift()!;
      } else if (streamEnded) {
        break;
      } else {
        yield await new Promise<T>((resolve, reject) => {
          resolveNext = (result) => {
            if (result.done) {
              resolve(undefined as unknown as T);
            } else {
              resolve(result.value);
            }
          };
          rejectNext = reject;
        });
      }
    }
  }

  /**
   * Fetch the next batch of rows
   */
  private async fetchNextBatch(): Promise<void> {
    if (this.ended || this.destroyed || this.paused) {
      return;
    }

    if (this.rowsStreamed >= this.options.maxRows) {
      this.end();
      return;
    }

    try {
      const batchSize = Math.min(
        this.options.batchSize,
        this.options.maxRows - this.rowsStreamed
      );

      const result = await this.client['post']<{
        rows: T[];
        has_more: boolean;
        row_count: number;
      }>('/api/v1/query/stream', {
        sql: this.sql,
        params: this.params,
        offset: this.offset,
        limit: batchSize,
        timeout: this.options.timeout,
      });

      if (result.rows.length === 0) {
        this.end();
        return;
      }

      this.batchesFetched++;
      this.offset += result.rows.length;

      // Emit batch event
      this.emit('batch', result.rows, this.batchesFetched);

      // Emit individual row events
      for (const row of result.rows) {
        this.rowsStreamed++;
        this.emit('data', row);

        // Apply back pressure if buffer is too large
        if (this.rowsStreamed % this.options.highWaterMark === 0) {
          await this.waitForDrain();
        }
      }

      // Emit stats periodically
      if (this.batchesFetched % 10 === 0) {
        this.emit('stats', this.getStats());
      }

      // Check if there are more rows
      if (!result.has_more || result.rows.length < batchSize) {
        this.end();
      } else {
        // Schedule next batch
        setImmediate(() => this.fetchNextBatch());
      }
    } catch (error) {
      this.emit('error', error instanceof Error ? error : new Error(String(error)));
      this.destroy();
    }
  }

  /**
   * Wait for buffer to drain (back pressure mechanism)
   */
  private async waitForDrain(): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, 10));
  }

  /**
   * End the stream
   */
  private end(): void {
    if (!this.ended) {
      this.ended = true;
      this.emit('stats', this.getStats());
      this.emit('end');
    }
  }
}

/**
 * Stream manager
 *
 * Manages multiple concurrent streams with resource limits.
 */
export class StreamManager {
  private client: BaseClient;
  private activeStreams: Set<QueryResultStream> = new Set();
  private maxConcurrentStreams: number;

  constructor(client: BaseClient, maxConcurrentStreams: number = 10) {
    this.client = client;
    this.maxConcurrentStreams = maxConcurrentStreams;
  }

  /**
   * Create a new query result stream
   *
   * @param sql - SQL query
   * @param params - Query parameters
   * @param options - Stream options
   * @returns Query result stream
   *
   * @example
   * ```typescript
   * const manager = new StreamManager(client);
   * const stream = manager.createStream('SELECT * FROM large_table', [], {
   *   batchSize: 500,
   *   maxRows: 1000000
   * });
   *
   * let count = 0;
   * stream.on('data', row => count++);
   * stream.on('end', () => console.log(`Processed ${count} rows`));
   * stream.on('stats', stats => console.log(`Speed: ${stats.rowsPerSecond.toFixed(0)} rows/s`));
   * stream.start();
   * ```
   */
  createStream<T = unknown>(
    sql: string,
    params: unknown[] = [],
    options: StreamOptions = {}
  ): QueryResultStream<T> {
    if (this.activeStreams.size >= this.maxConcurrentStreams) {
      throw new Error(
        `Maximum concurrent streams (${this.maxConcurrentStreams}) reached`
      );
    }

    const stream = new QueryResultStream<T>(this.client, sql, params, options);

    this.activeStreams.add(stream);

    stream.on('end', () => this.activeStreams.delete(stream));
    stream.on('error', () => this.activeStreams.delete(stream));

    return stream;
  }

  /**
   * Get the number of active streams
   */
  getActiveStreamCount(): number {
    return this.activeStreams.size;
  }

  /**
   * Destroy all active streams
   */
  destroyAll(): void {
    for (const stream of this.activeStreams) {
      stream.destroy();
    }
    this.activeStreams.clear();
  }
}

/**
 * Utility function to stream query results
 *
 * @param client - Base client
 * @param sql - SQL query
 * @param params - Query parameters
 * @param options - Stream options
 * @returns Async iterable of rows
 *
 * @example
 * ```typescript
 * for await (const row of streamQuery(client, 'SELECT * FROM users')) {
 *   console.log(row);
 * }
 * ```
 */
export async function* streamQuery<T = unknown>(
  client: BaseClient,
  sql: string,
  params: unknown[] = [],
  options: StreamOptions = {}
): AsyncIterable<T> {
  const stream = new QueryResultStream<T>(client, sql, params, options);
  yield* stream;
}

export default {
  QueryResultStream,
  StreamManager,
  streamQuery,
};
