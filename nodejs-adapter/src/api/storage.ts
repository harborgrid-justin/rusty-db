/**
 * Storage & Buffer Pool API Client
 *
 * Provides TypeScript client methods for all RustyDB storage-related REST endpoints.
 * This includes disk management, partitions, buffer pool, tablespaces, and I/O statistics.
 *
 * @module api/storage
 */

// ============================================================================
// Type Definitions (mapped from Rust types in storage_handlers.rs)
// ============================================================================

/**
 * Overall storage status across all disks and tablespaces
 */
export interface StorageStatus {
  total_space_bytes: number;
  used_space_bytes: number;
  available_space_bytes: number;
  utilization_percent: number;
  disk_count: number;
  partition_count: number;
  tablespace_count: number;
}

/**
 * Disk device information including I/O metrics
 */
export interface DiskInfo {
  disk_id: string;
  device_path: string;
  mount_point: string;
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  read_iops: number;
  write_iops: number;
  read_throughput_mbps: number;
  write_throughput_mbps: number;
  avg_latency_ms: number;
}

/**
 * Table partition information
 */
export interface PartitionInfo {
  partition_id: string;
  table_name: string;
  partition_name: string;
  partition_type: 'range' | 'list' | 'hash';
  partition_key: string;
  partition_value: string;
  row_count: number;
  size_bytes: number;
  created_at: number; // Unix timestamp
}

/**
 * Request to create a new partition
 */
export interface CreatePartitionRequest {
  table_name: string;
  partition_name: string;
  partition_type: 'range' | 'list' | 'hash';
  partition_key: string;
  partition_value: string;
}

/**
 * Buffer pool statistics and metrics
 */
export interface BufferPoolStats {
  total_pages: number;
  used_pages: number;
  free_pages: number;
  dirty_pages: number;
  hit_ratio: number;
  evictions: number;
  reads: number;
  writes: number;
  flushes: number;
}

/**
 * Response from buffer pool flush operation
 */
export interface BufferPoolFlushResponse {
  status: 'success' | 'failed';
  pages_flushed: number;
  timestamp: number; // Unix timestamp
}

/**
 * Tablespace information
 */
export interface TablespaceInfo {
  tablespace_id: string;
  name: string;
  location: string;
  size_bytes: number;
  used_bytes: number;
  auto_extend: boolean;
  max_size_bytes: number | null;
  status: 'online' | 'offline';
}

/**
 * Request to create a new tablespace
 */
export interface CreateTablespaceRequest {
  name: string;
  location: string;
  initial_size_mb: number;
  auto_extend?: boolean;
  max_size_mb?: number;
}

/**
 * Request to update an existing tablespace
 */
export interface UpdateTablespaceRequest {
  auto_extend?: boolean;
  max_size_mb?: number;
  status?: 'online' | 'offline';
}

/**
 * I/O statistics for the storage subsystem
 */
export interface IoStats {
  total_reads: number;
  total_writes: number;
  bytes_read: number;
  bytes_written: number;
  avg_read_latency_ms: number;
  avg_write_latency_ms: number;
  read_iops: number;
  write_iops: number;
  timestamp: number; // Unix timestamp
}

/**
 * Error response from the API
 */
export interface ApiError {
  code: string;
  message: string;
}

// ============================================================================
// Storage API Client
// ============================================================================

/**
 * Client configuration
 */
export interface StorageClientConfig {
  baseUrl: string;
  apiVersion?: string;
  timeout?: number;
  headers?: Record<string, string>;
}

/**
 * Storage API Client
 *
 * Provides methods for interacting with all storage-related endpoints.
 */
export class StorageClient {
  private baseUrl: string;
  private apiVersion: string;
  private timeout: number;
  private headers: Record<string, string>;

  constructor(config: StorageClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/$/, ''); // Remove trailing slash
    this.apiVersion = config.apiVersion || 'v1';
    this.timeout = config.timeout || 30000;
    this.headers = config.headers || {};
  }

  /**
   * Build the full URL for an endpoint
   */
  private buildUrl(path: string): string {
    return `${this.baseUrl}/api/${this.apiVersion}${path}`;
  }

  /**
   * Make an HTTP request with error handling
   */
  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const url = this.buildUrl(path);
    const options: RequestInit = {
      method,
      headers: {
        'Content-Type': 'application/json',
        ...this.headers,
      },
      signal: AbortSignal.timeout(this.timeout),
    };

    if (body !== undefined) {
      options.body = JSON.stringify(body);
    }

    try {
      const response = await fetch(url, options);

      if (!response.ok) {
        const error: ApiError = await response.json().catch(() => ({
          code: 'UNKNOWN_ERROR',
          message: `HTTP ${response.status}: ${response.statusText}`,
        }));
        throw new Error(`[${error.code}] ${error.message}`);
      }

      // Handle 204 No Content responses
      if (response.status === 204) {
        return undefined as T;
      }

      return await response.json();
    } catch (error) {
      if (error instanceof Error) {
        throw error;
      }
      throw new Error(`Request failed: ${String(error)}`);
    }
  }

  // ============================================================================
  // Storage Status & Disks
  // ============================================================================

  /**
   * Get overall storage status
   *
   * @returns Overall storage statistics across all disks
   *
   * @example
   * const status = await client.getStorageStatus();
   * console.log(`Storage utilization: ${status.utilization_percent}%`);
   */
  async getStorageStatus(): Promise<StorageStatus> {
    return this.request<StorageStatus>('GET', '/storage/status');
  }

  /**
   * List all disk devices and their statistics
   *
   * @returns Array of disk information
   *
   * @example
   * const disks = await client.getDisks();
   * disks.forEach(disk => {
   *   console.log(`${disk.disk_id}: ${disk.read_iops} read IOPS`);
   * });
   */
  async getDisks(): Promise<DiskInfo[]> {
    return this.request<DiskInfo[]>('GET', '/storage/disks');
  }

  /**
   * Get I/O statistics for the storage subsystem
   *
   * @returns Current I/O statistics
   *
   * @example
   * const stats = await client.getIoStats();
   * console.log(`Read IOPS: ${stats.read_iops}`);
   * console.log(`Write IOPS: ${stats.write_iops}`);
   */
  async getIoStats(): Promise<IoStats> {
    return this.request<IoStats>('GET', '/storage/io-stats');
  }

  // ============================================================================
  // Partitions
  // ============================================================================

  /**
   * List all partitions
   *
   * @returns Array of partition information
   *
   * @example
   * const partitions = await client.getPartitions();
   * console.log(`Total partitions: ${partitions.length}`);
   */
  async getPartitions(): Promise<PartitionInfo[]> {
    return this.request<PartitionInfo[]>('GET', '/storage/partitions');
  }

  /**
   * Create a new partition
   *
   * @param request - Partition creation parameters
   * @returns Created partition information
   *
   * @example
   * const partition = await client.createPartition({
   *   table_name: 'sales',
   *   partition_name: 'sales_2024_q1',
   *   partition_type: 'range',
   *   partition_key: 'sale_date',
   *   partition_value: '2024-01-01 TO 2024-03-31'
   * });
   * console.log(`Created partition: ${partition.partition_id}`);
   */
  async createPartition(request: CreatePartitionRequest): Promise<PartitionInfo> {
    return this.request<PartitionInfo>('POST', '/storage/partitions', request);
  }

  /**
   * Delete a partition
   *
   * @param partitionId - Partition ID to delete
   *
   * @example
   * await client.deletePartition('part_123');
   * console.log('Partition deleted successfully');
   */
  async deletePartition(partitionId: string): Promise<void> {
    return this.request<void>('DELETE', `/storage/partitions/${partitionId}`);
  }

  // ============================================================================
  // Buffer Pool
  // ============================================================================

  /**
   * Get buffer pool statistics
   *
   * @returns Current buffer pool statistics
   *
   * @example
   * const stats = await client.getBufferPoolStats();
   * console.log(`Hit ratio: ${(stats.hit_ratio * 100).toFixed(2)}%`);
   * console.log(`Dirty pages: ${stats.dirty_pages}`);
   */
  async getBufferPoolStats(): Promise<BufferPoolStats> {
    return this.request<BufferPoolStats>('GET', '/storage/buffer-pool');
  }

  /**
   * Flush buffer pool to disk
   *
   * @returns Flush operation result
   *
   * @example
   * const result = await client.flushBufferPool();
   * console.log(`Flushed ${result.pages_flushed} pages`);
   */
  async flushBufferPool(): Promise<BufferPoolFlushResponse> {
    return this.request<BufferPoolFlushResponse>('POST', '/storage/buffer-pool/flush');
  }

  // ============================================================================
  // Tablespaces
  // ============================================================================

  /**
   * List all tablespaces
   *
   * @returns Array of tablespace information
   *
   * @example
   * const tablespaces = await client.getTablespaces();
   * tablespaces.forEach(ts => {
   *   console.log(`${ts.name}: ${ts.status}`);
   * });
   */
  async getTablespaces(): Promise<TablespaceInfo[]> {
    return this.request<TablespaceInfo[]>('GET', '/storage/tablespaces');
  }

  /**
   * Create a new tablespace
   *
   * @param request - Tablespace creation parameters
   * @returns Created tablespace information
   *
   * @example
   * const tablespace = await client.createTablespace({
   *   name: 'user_data',
   *   location: '/data/tablespaces/user_data',
   *   initial_size_mb: 1024,
   *   auto_extend: true,
   *   max_size_mb: 10240
   * });
   * console.log(`Created tablespace: ${tablespace.tablespace_id}`);
   */
  async createTablespace(request: CreateTablespaceRequest): Promise<TablespaceInfo> {
    return this.request<TablespaceInfo>('POST', '/storage/tablespaces', request);
  }

  /**
   * Update an existing tablespace
   *
   * @param tablespaceId - Tablespace ID/name to update
   * @param request - Update parameters
   * @returns Updated tablespace information
   *
   * @example
   * const updated = await client.updateTablespace('system', {
   *   max_size_mb: 20480,
   *   auto_extend: true
   * });
   * console.log(`Updated tablespace: ${updated.name}`);
   */
  async updateTablespace(
    tablespaceId: string,
    request: UpdateTablespaceRequest
  ): Promise<TablespaceInfo> {
    return this.request<TablespaceInfo>(
      'PUT',
      `/storage/tablespaces/${tablespaceId}`,
      request
    );
  }

  /**
   * Delete a tablespace
   *
   * @param tablespaceId - Tablespace ID/name to delete
   *
   * @example
   * await client.deleteTablespace('old_data');
   * console.log('Tablespace deleted successfully');
   */
  async deleteTablespace(tablespaceId: string): Promise<void> {
    return this.request<void>('DELETE', `/storage/tablespaces/${tablespaceId}`);
  }

  // ============================================================================
  // Convenience Methods
  // ============================================================================

  /**
   * Get storage utilization as a percentage
   *
   * @returns Storage utilization percentage (0-100)
   *
   * @example
   * const utilization = await client.getStorageUtilization();
   * if (utilization > 90) {
   *   console.warn(`Storage is ${utilization}% full!`);
   * }
   */
  async getStorageUtilization(): Promise<number> {
    const status = await this.getStorageStatus();
    return status.utilization_percent;
  }

  /**
   * Get buffer pool hit ratio as a percentage
   *
   * @returns Buffer pool hit ratio (0-100)
   *
   * @example
   * const hitRatio = await client.getBufferPoolHitRatio();
   * console.log(`Buffer pool efficiency: ${hitRatio.toFixed(2)}%`);
   */
  async getBufferPoolHitRatio(): Promise<number> {
    const stats = await this.getBufferPoolStats();
    return stats.hit_ratio * 100;
  }

  /**
   * Get total I/O throughput in MB/s
   *
   * @returns Total I/O throughput
   *
   * @example
   * const disks = await client.getDisks();
   * const totalThroughput = disks.reduce((sum, disk) =>
   *   sum + disk.read_throughput_mbps + disk.write_throughput_mbps, 0
   * );
   */
  async getTotalIoThroughput(): Promise<{
    read_mbps: number;
    write_mbps: number;
    total_mbps: number;
  }> {
    const disks = await this.getDisks();
    const read_mbps = disks.reduce((sum, disk) => sum + disk.read_throughput_mbps, 0);
    const write_mbps = disks.reduce((sum, disk) => sum + disk.write_throughput_mbps, 0);
    return {
      read_mbps,
      write_mbps,
      total_mbps: read_mbps + write_mbps,
    };
  }

  /**
   * Get partitions for a specific table
   *
   * @param tableName - Table name to filter by
   * @returns Array of partitions for the specified table
   *
   * @example
   * const salesPartitions = await client.getPartitionsByTable('sales');
   * console.log(`Table 'sales' has ${salesPartitions.length} partitions`);
   */
  async getPartitionsByTable(tableName: string): Promise<PartitionInfo[]> {
    const partitions = await this.getPartitions();
    return partitions.filter(p => p.table_name === tableName);
  }

  /**
   * Get tablespace by name
   *
   * @param name - Tablespace name
   * @returns Tablespace information or null if not found
   *
   * @example
   * const system = await client.getTablespaceByName('system');
   * if (system) {
   *   console.log(`System tablespace is ${system.status}`);
   * }
   */
  async getTablespaceByName(name: string): Promise<TablespaceInfo | null> {
    const tablespaces = await this.getTablespaces();
    return tablespaces.find(ts => ts.name === name) || null;
  }
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a new StorageClient instance
 *
 * @param config - Client configuration
 * @returns Configured StorageClient instance
 *
 * @example
 * const client = createStorageClient({
 *   baseUrl: 'http://localhost:5432',
 *   timeout: 30000,
 *   headers: { 'Authorization': 'Bearer token123' }
 * });
 */
export function createStorageClient(config: StorageClientConfig): StorageClient {
  return new StorageClient(config);
}

// ============================================================================
// Default Export
// ============================================================================

export default StorageClient;
