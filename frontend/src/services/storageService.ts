import { get, post, del, buildQueryParams } from './api';
import type {
  ApiResponse,
  UUID,
  Timestamp,
  PaginationParams,
  PaginatedResponse,
} from '@/types';

// ============================================================================
// Storage Service Types
// ============================================================================

export interface StorageStatus {
  totalDiskSpace: number;
  usedDiskSpace: number;
  availableDiskSpace: number;
  usagePercent: number;
  totalPages: number;
  usedPages: number;
  freePages: number;
  dirtyPages: number;
  lastCheckpoint: Timestamp;
  checkpointLag: number;
}

export interface DiskDevice {
  id: string;
  name: string;
  path: string;
  type: 'ssd' | 'hdd' | 'nvme' | 'network';
  totalSize: number;
  usedSize: number;
  availableSize: number;
  usagePercent: number;
  mounted: boolean;
  readOps: number;
  writeOps: number;
  readBytes: number;
  writeBytes: number;
  avgReadLatency: number;
  avgWriteLatency: number;
  health: 'healthy' | 'degraded' | 'critical' | 'failed';
}

export interface Partition {
  id: UUID;
  name: string;
  tableName: string;
  schema: string;
  partitionType: PartitionType;
  partitionKey: string;
  partitionValue?: string;
  rangeStart?: string;
  rangeEnd?: string;
  rowCount: number;
  size: number;
  indexCount: number;
  isEnabled: boolean;
  createdAt: Timestamp;
  updatedAt: Timestamp;
  lastAccessed?: Timestamp;
}

export type PartitionType = 'range' | 'hash' | 'list' | 'composite';

export interface CreatePartitionRequest {
  tableName: string;
  schema: string;
  name: string;
  partitionType: PartitionType;
  partitionKey: string;
  partitionValue?: string;
  rangeStart?: string;
  rangeEnd?: string;
}

export interface BufferPoolStats {
  totalPages: number;
  usedPages: number;
  freePages: number;
  dirtyPages: number;
  pinnedPages: number;
  usagePercent: number;
  hitRate: number;
  missRate: number;
  evictions: number;
  evictionPolicy: 'clock' | 'lru' | '2q' | 'lru-k' | 'lirs' | 'arc';
  readRequests: number;
  writeRequests: number;
  flushes: number;
  lastFlushTime?: Timestamp;
}

export interface FlushBufferPoolResponse {
  flushedPages: number;
  duration: number;
  startTime: Timestamp;
  endTime: Timestamp;
}

export interface Tablespace {
  id: UUID;
  name: string;
  owner: string;
  location: string;
  size: number;
  usedSize: number;
  availableSize: number;
  usagePercent: number;
  tableCount: number;
  indexCount: number;
  isDefault: boolean;
  isOnline: boolean;
  encrypted: boolean;
  compressionEnabled: boolean;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface CreateTablespaceRequest {
  name: string;
  location: string;
  encrypted?: boolean;
  compressionEnabled?: boolean;
  maxSize?: number;
}

export interface PartitionFilters extends Partial<PaginationParams> {
  tableName?: string;
  schema?: string;
  partitionType?: PartitionType;
  isEnabled?: boolean;
}

// ============================================================================
// Storage Service
// ============================================================================

export const storageService = {
  // ============================================================================
  // Storage Status
  // ============================================================================

  /**
   * Get overall storage system status
   */
  async getStorageStatus(): Promise<StorageStatus> {
    const response = await get<StorageStatus>('/storage/status');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch storage status');
    }

    return response.data;
  },

  // ============================================================================
  // Disk Devices
  // ============================================================================

  /**
   * List all disk devices
   */
  async getDisks(): Promise<DiskDevice[]> {
    const response = await get<DiskDevice[]>('/storage/disks');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch disk devices');
    }

    return response.data;
  },

  /**
   * Get a single disk device by ID
   */
  async getDisk(id: string): Promise<DiskDevice> {
    const response = await get<DiskDevice>(`/storage/disks/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch disk device');
    }

    return response.data;
  },

  // ============================================================================
  // Partitions
  // ============================================================================

  /**
   * List partitions with optional filters
   */
  async getPartitions(filters?: PartitionFilters): Promise<PaginatedResponse<Partition>> {
    const queryParams = filters ? buildQueryParams(filters) : '';
    const response = await get<PaginatedResponse<Partition>>(`/storage/partitions${queryParams}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch partitions');
    }

    return response.data;
  },

  /**
   * Get a single partition by ID
   */
  async getPartition(id: UUID): Promise<Partition> {
    const response = await get<Partition>(`/storage/partitions/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch partition');
    }

    return response.data;
  },

  /**
   * Create a new table partition
   */
  async createPartition(request: CreatePartitionRequest): Promise<Partition> {
    const response = await post<Partition>('/storage/partitions', request);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to create partition');
    }

    return response.data;
  },

  /**
   * Delete a partition
   */
  async deletePartition(id: UUID): Promise<void> {
    const response = await del(`/storage/partitions/${id}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to delete partition');
    }
  },

  // ============================================================================
  // Buffer Pool
  // ============================================================================

  /**
   * Get buffer pool statistics
   */
  async getBufferPoolStats(): Promise<BufferPoolStats> {
    const response = await get<BufferPoolStats>('/storage/buffer-pool');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch buffer pool stats');
    }

    return response.data;
  },

  /**
   * Flush dirty pages from buffer pool to disk
   */
  async flushBufferPool(): Promise<FlushBufferPoolResponse> {
    const response = await post<FlushBufferPoolResponse>('/storage/buffer-pool/flush');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to flush buffer pool');
    }

    return response.data;
  },

  // ============================================================================
  // Tablespaces
  // ============================================================================

  /**
   * List all tablespaces
   */
  async getTablespaces(): Promise<Tablespace[]> {
    const response = await get<Tablespace[]>('/storage/tablespaces');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch tablespaces');
    }

    return response.data;
  },

  /**
   * Get a single tablespace by ID
   */
  async getTablespace(id: UUID): Promise<Tablespace> {
    const response = await get<Tablespace>(`/storage/tablespaces/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch tablespace');
    }

    return response.data;
  },

  /**
   * Create a new tablespace
   */
  async createTablespace(request: CreateTablespaceRequest): Promise<Tablespace> {
    const response = await post<Tablespace>('/storage/tablespaces', request);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to create tablespace');
    }

    return response.data;
  },

  /**
   * Delete a tablespace
   */
  async deleteTablespace(id: UUID): Promise<void> {
    const response = await del(`/storage/tablespaces/${id}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to delete tablespace');
    }
  },
};
