/**
 * Storage & Buffer Pool API Client Tests
 *
 * Comprehensive test suite for all storage-related REST endpoints.
 *
 * @module test/storage
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  StorageClient,
  createStorageClient,
  StorageStatus,
  DiskInfo,
  PartitionInfo,
  BufferPoolStats,
  BufferPoolFlushResponse,
  TablespaceInfo,
  IoStats,
  CreatePartitionRequest,
  CreateTablespaceRequest,
  UpdateTablespaceRequest,
} from '../src/api/storage';

// ============================================================================
// Mock Data
// ============================================================================

const mockStorageStatus: StorageStatus = {
  total_space_bytes: 1_000_000_000_000,
  used_space_bytes: 500_000_000_000,
  available_space_bytes: 500_000_000_000,
  utilization_percent: 50.0,
  disk_count: 1,
  partition_count: 5,
  tablespace_count: 2,
};

const mockDiskInfo: DiskInfo = {
  disk_id: 'disk0',
  device_path: '/dev/sda1',
  mount_point: '/data',
  total_bytes: 1_000_000_000_000,
  used_bytes: 500_000_000_000,
  available_bytes: 500_000_000_000,
  read_iops: 1000,
  write_iops: 800,
  read_throughput_mbps: 150.5,
  write_throughput_mbps: 120.3,
  avg_latency_ms: 2.5,
};

const mockPartitionInfo: PartitionInfo = {
  partition_id: 'part_1',
  table_name: 'sales',
  partition_name: 'sales_2024_q1',
  partition_type: 'range',
  partition_key: 'sale_date',
  partition_value: '2024-01-01 TO 2024-03-31',
  row_count: 100000,
  size_bytes: 50_000_000,
  created_at: 1704067200,
};

const mockBufferPoolStats: BufferPoolStats = {
  total_pages: 10000,
  used_pages: 7500,
  free_pages: 2500,
  dirty_pages: 500,
  hit_ratio: 0.95,
  evictions: 1000,
  reads: 50000,
  writes: 25000,
  flushes: 500,
};

const mockBufferPoolFlushResponse: BufferPoolFlushResponse = {
  status: 'success',
  pages_flushed: 500,
  timestamp: 1704067200,
};

const mockTablespaceInfo: TablespaceInfo = {
  tablespace_id: 'ts_system',
  name: 'system',
  location: '/data/tablespaces/system',
  size_bytes: 10_000_000_000,
  used_bytes: 5_000_000_000,
  auto_extend: true,
  max_size_bytes: 50_000_000_000,
  status: 'online',
};

const mockIoStats: IoStats = {
  total_reads: 1_000_000,
  total_writes: 500_000,
  bytes_read: 100_000_000_000,
  bytes_written: 50_000_000_000,
  avg_read_latency_ms: 2.5,
  avg_write_latency_ms: 3.2,
  read_iops: 1500.0,
  write_iops: 800.0,
  timestamp: 1704067200,
};

// ============================================================================
// Test Suite
// ============================================================================

describe('StorageClient', () => {
  let client: StorageClient;
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    // Create a fresh client for each test
    client = createStorageClient({
      baseUrl: 'http://localhost:5432',
      apiVersion: 'v1',
      timeout: 5000,
    });

    // Mock fetch
    fetchMock = vi.fn();
    global.fetch = fetchMock;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ============================================================================
  // Storage Status & Disks Tests
  // ============================================================================

  describe('getStorageStatus', () => {
    it('should fetch storage status successfully', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockStorageStatus,
      });

      const result = await client.getStorageStatus();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/status',
        expect.objectContaining({
          method: 'GET',
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
          }),
        })
      );
      expect(result).toEqual(mockStorageStatus);
      expect(result.utilization_percent).toBe(50.0);
    });

    it('should handle errors when fetching storage status', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        json: async () => ({ code: 'INTERNAL_ERROR', message: 'Server error' }),
      });

      await expect(client.getStorageStatus()).rejects.toThrow(
        '[INTERNAL_ERROR] Server error'
      );
    });
  });

  describe('getDisks', () => {
    it('should fetch list of disks successfully', async () => {
      const mockDisks = [mockDiskInfo];
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockDisks,
      });

      const result = await client.getDisks();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/disks',
        expect.any(Object)
      );
      expect(result).toEqual(mockDisks);
      expect(result).toHaveLength(1);
      expect(result[0].disk_id).toBe('disk0');
    });
  });

  describe('getIoStats', () => {
    it('should fetch I/O statistics successfully', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockIoStats,
      });

      const result = await client.getIoStats();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/io-stats',
        expect.any(Object)
      );
      expect(result).toEqual(mockIoStats);
      expect(result.read_iops).toBe(1500.0);
      expect(result.write_iops).toBe(800.0);
    });
  });

  // ============================================================================
  // Partitions Tests
  // ============================================================================

  describe('getPartitions', () => {
    it('should fetch list of partitions successfully', async () => {
      const mockPartitions = [mockPartitionInfo];
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockPartitions,
      });

      const result = await client.getPartitions();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/partitions',
        expect.any(Object)
      );
      expect(result).toEqual(mockPartitions);
      expect(result).toHaveLength(1);
    });
  });

  describe('createPartition', () => {
    it('should create a partition successfully', async () => {
      const request: CreatePartitionRequest = {
        table_name: 'sales',
        partition_name: 'sales_2024_q2',
        partition_type: 'range',
        partition_key: 'sale_date',
        partition_value: '2024-04-01 TO 2024-06-30',
      };

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 201,
        json: async () => ({ ...mockPartitionInfo, ...request }),
      });

      const result = await client.createPartition(request);

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/partitions',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(request),
        })
      );
      expect(result.partition_name).toBe('sales_2024_q2');
    });

    it('should handle validation errors when creating partition', async () => {
      const request: CreatePartitionRequest = {
        table_name: '',
        partition_name: 'invalid',
        partition_type: 'range',
        partition_key: 'date',
        partition_value: 'invalid',
      };

      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 400,
        json: async () => ({
          code: 'VALIDATION_ERROR',
          message: 'Table name cannot be empty',
        }),
      });

      await expect(client.createPartition(request)).rejects.toThrow(
        '[VALIDATION_ERROR] Table name cannot be empty'
      );
    });
  });

  describe('deletePartition', () => {
    it('should delete a partition successfully', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 204,
      });

      await client.deletePartition('part_1');

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/partitions/part_1',
        expect.objectContaining({
          method: 'DELETE',
        })
      );
    });

    it('should handle not found error when deleting partition', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 404,
        json: async () => ({
          code: 'NOT_FOUND',
          message: 'Partition part_999 not found',
        }),
      });

      await expect(client.deletePartition('part_999')).rejects.toThrow(
        '[NOT_FOUND] Partition part_999 not found'
      );
    });
  });

  // ============================================================================
  // Buffer Pool Tests
  // ============================================================================

  describe('getBufferPoolStats', () => {
    it('should fetch buffer pool statistics successfully', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockBufferPoolStats,
      });

      const result = await client.getBufferPoolStats();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/buffer-pool',
        expect.any(Object)
      );
      expect(result).toEqual(mockBufferPoolStats);
      expect(result.hit_ratio).toBe(0.95);
      expect(result.dirty_pages).toBe(500);
    });
  });

  describe('flushBufferPool', () => {
    it('should flush buffer pool successfully', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockBufferPoolFlushResponse,
      });

      const result = await client.flushBufferPool();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/buffer-pool/flush',
        expect.objectContaining({
          method: 'POST',
        })
      );
      expect(result.status).toBe('success');
      expect(result.pages_flushed).toBe(500);
    });
  });

  // ============================================================================
  // Tablespaces Tests
  // ============================================================================

  describe('getTablespaces', () => {
    it('should fetch list of tablespaces successfully', async () => {
      const mockTablespaces = [mockTablespaceInfo];
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockTablespaces,
      });

      const result = await client.getTablespaces();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/tablespaces',
        expect.any(Object)
      );
      expect(result).toEqual(mockTablespaces);
      expect(result).toHaveLength(1);
      expect(result[0].name).toBe('system');
    });
  });

  describe('createTablespace', () => {
    it('should create a tablespace successfully', async () => {
      const request: CreateTablespaceRequest = {
        name: 'user_data',
        location: '/data/tablespaces/user_data',
        initial_size_mb: 1024,
        auto_extend: true,
        max_size_mb: 10240,
      };

      const expectedResponse: TablespaceInfo = {
        tablespace_id: 'ts_user_data',
        name: 'user_data',
        location: '/data/tablespaces/user_data',
        size_bytes: 1024 * 1024 * 1024,
        used_bytes: 0,
        auto_extend: true,
        max_size_bytes: 10240 * 1024 * 1024,
        status: 'online',
      };

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 201,
        json: async () => expectedResponse,
      });

      const result = await client.createTablespace(request);

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/tablespaces',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(request),
        })
      );
      expect(result.name).toBe('user_data');
      expect(result.status).toBe('online');
    });
  });

  describe('updateTablespace', () => {
    it('should update a tablespace successfully', async () => {
      const request: UpdateTablespaceRequest = {
        auto_extend: false,
        max_size_mb: 20480,
      };

      const updatedTablespace: TablespaceInfo = {
        ...mockTablespaceInfo,
        auto_extend: false,
        max_size_bytes: 20480 * 1024 * 1024,
      };

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => updatedTablespace,
      });

      const result = await client.updateTablespace('system', request);

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/tablespaces/system',
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(request),
        })
      );
      expect(result.auto_extend).toBe(false);
      expect(result.max_size_bytes).toBe(20480 * 1024 * 1024);
    });

    it('should handle not found error when updating tablespace', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 404,
        json: async () => ({
          code: 'NOT_FOUND',
          message: 'Tablespace unknown not found',
        }),
      });

      await expect(
        client.updateTablespace('unknown', { status: 'offline' })
      ).rejects.toThrow('[NOT_FOUND] Tablespace unknown not found');
    });
  });

  describe('deleteTablespace', () => {
    it('should delete a tablespace successfully', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 204,
      });

      await client.deleteTablespace('old_data');

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/tablespaces/old_data',
        expect.objectContaining({
          method: 'DELETE',
        })
      );
    });
  });

  // ============================================================================
  // Convenience Methods Tests
  // ============================================================================

  describe('getStorageUtilization', () => {
    it('should return storage utilization percentage', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockStorageStatus,
      });

      const result = await client.getStorageUtilization();

      expect(result).toBe(50.0);
    });
  });

  describe('getBufferPoolHitRatio', () => {
    it('should return buffer pool hit ratio as percentage', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockBufferPoolStats,
      });

      const result = await client.getBufferPoolHitRatio();

      expect(result).toBe(95.0);
    });
  });

  describe('getTotalIoThroughput', () => {
    it('should calculate total I/O throughput', async () => {
      const mockDisks = [
        mockDiskInfo,
        { ...mockDiskInfo, disk_id: 'disk1' },
      ];

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockDisks,
      });

      const result = await client.getTotalIoThroughput();

      expect(result.read_mbps).toBe(301.0); // 150.5 * 2
      expect(result.write_mbps).toBe(240.6); // 120.3 * 2
      expect(result.total_mbps).toBe(541.6);
    });
  });

  describe('getPartitionsByTable', () => {
    it('should filter partitions by table name', async () => {
      const mockPartitions = [
        mockPartitionInfo,
        { ...mockPartitionInfo, partition_id: 'part_2', table_name: 'orders' },
        { ...mockPartitionInfo, partition_id: 'part_3', table_name: 'sales' },
      ];

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockPartitions,
      });

      const result = await client.getPartitionsByTable('sales');

      expect(result).toHaveLength(2);
      expect(result.every(p => p.table_name === 'sales')).toBe(true);
    });
  });

  describe('getTablespaceByName', () => {
    it('should find tablespace by name', async () => {
      const mockTablespaces = [
        mockTablespaceInfo,
        { ...mockTablespaceInfo, tablespace_id: 'ts_user', name: 'user_data' },
      ];

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockTablespaces,
      });

      const result = await client.getTablespaceByName('system');

      expect(result).not.toBeNull();
      expect(result?.name).toBe('system');
    });

    it('should return null when tablespace not found', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => [mockTablespaceInfo],
      });

      const result = await client.getTablespaceByName('nonexistent');

      expect(result).toBeNull();
    });
  });

  // ============================================================================
  // Configuration Tests
  // ============================================================================

  describe('Client Configuration', () => {
    it('should use custom base URL', () => {
      const customClient = createStorageClient({
        baseUrl: 'http://custom.example.com:8080',
      });

      expect(customClient).toBeInstanceOf(StorageClient);
    });

    it('should handle trailing slash in base URL', async () => {
      const customClient = createStorageClient({
        baseUrl: 'http://localhost:5432/',
      });

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockStorageStatus,
      });

      await customClient.getStorageStatus();

      expect(fetchMock).toHaveBeenCalledWith(
        'http://localhost:5432/api/v1/storage/status',
        expect.any(Object)
      );
    });

    it('should include custom headers', async () => {
      const customClient = createStorageClient({
        baseUrl: 'http://localhost:5432',
        headers: { Authorization: 'Bearer token123' },
      });

      fetchMock.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockStorageStatus,
      });

      await customClient.getStorageStatus();

      expect(fetchMock).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            Authorization: 'Bearer token123',
          }),
        })
      );
    });
  });

  // ============================================================================
  // Error Handling Tests
  // ============================================================================

  describe('Error Handling', () => {
    it('should handle network errors', async () => {
      fetchMock.mockRejectedValueOnce(new Error('Network failure'));

      await expect(client.getStorageStatus()).rejects.toThrow('Network failure');
    });

    it('should handle timeout errors', async () => {
      fetchMock.mockImplementationOnce(
        () =>
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error('Timeout')), 100)
          )
      );

      await expect(client.getStorageStatus()).rejects.toThrow();
    });

    it('should handle malformed JSON responses', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        json: async () => {
          throw new Error('Invalid JSON');
        },
      });

      await expect(client.getStorageStatus()).rejects.toThrow();
    });
  });
});
