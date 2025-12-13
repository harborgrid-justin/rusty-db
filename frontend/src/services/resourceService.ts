import { get, post, put, del } from './api';
import type {
  ResourceGroup,
  ResourceUsage,
  ConnectionPoolStats,
} from '../types';

// ============================================================================
// Resource Management Service
// API client for resource groups and connection pools
// ============================================================================

// ============================================================================
// Resource Group APIs
// ============================================================================

export interface CreateResourceGroupRequest {
  name: string;
  cpuLimit: number;
  memoryLimit: number;
  ioLimit?: number;
  maxConnections: number;
  maxQueries: number;
  queryTimeout: number;
  priority: number;
  members: string[];
  isEnabled: boolean;
}

export interface UpdateResourceGroupRequest {
  name?: string;
  cpuLimit?: number;
  memoryLimit?: number;
  ioLimit?: number;
  maxConnections?: number;
  maxQueries?: number;
  queryTimeout?: number;
  priority?: number;
  members?: string[];
  isEnabled?: boolean;
}

/**
 * Get all resource groups
 */
export async function getResourceGroups(): Promise<ResourceGroup[]> {
  const response = await get<ResourceGroup[]>('/resources/groups');
  return response.data || [];
}

/**
 * Get a specific resource group by ID
 */
export async function getResourceGroup(id: string): Promise<ResourceGroup> {
  const response = await get<ResourceGroup>(`/resources/groups/${id}`);
  if (!response.data) {
    throw new Error('Resource group not found');
  }
  return response.data;
}

/**
 * Create a new resource group
 */
export async function createResourceGroup(
  config: CreateResourceGroupRequest
): Promise<ResourceGroup> {
  const response = await post<ResourceGroup>('/resources/groups', config);
  if (!response.data) {
    throw new Error('Failed to create resource group');
  }
  return response.data;
}

/**
 * Update an existing resource group
 */
export async function updateResourceGroup(
  id: string,
  config: UpdateResourceGroupRequest
): Promise<ResourceGroup> {
  const response = await put<ResourceGroup>(`/resources/groups/${id}`, config);
  if (!response.data) {
    throw new Error('Failed to update resource group');
  }
  return response.data;
}

/**
 * Delete a resource group
 */
export async function deleteResourceGroup(id: string): Promise<void> {
  await del(`/resources/groups/${id}`);
}

/**
 * Get resource usage statistics for a specific group
 */
export async function getResourceUsage(groupId: string): Promise<ResourceUsage> {
  const response = await get<ResourceUsage>(`/resources/groups/${groupId}/usage`);
  if (!response.data) {
    throw new Error('Usage data not found');
  }
  return response.data;
}

/**
 * Get resource usage statistics for all groups
 */
export async function getAllResourceUsage(): Promise<ResourceUsage[]> {
  const response = await get<ResourceUsage[]>('/resources/usage');
  return response.data || [];
}

// ============================================================================
// Connection Pool APIs
// ============================================================================

export interface CreatePoolRequest {
  poolId: string;
  minConnections: number;
  maxConnections: number;
  connectionTimeout: number;
  idleTimeout: number;
  validationInterval: number;
}

export interface UpdatePoolRequest {
  minConnections?: number;
  maxConnections?: number;
  connectionTimeout?: number;
  idleTimeout?: number;
  validationInterval?: number;
}

/**
 * Get all connection pools
 */
export async function getConnectionPools(): Promise<ConnectionPoolStats[]> {
  const response = await get<ConnectionPoolStats[]>('/resources/pools');
  return response.data || [];
}

/**
 * Get a specific connection pool by ID
 */
export async function getConnectionPool(poolId: string): Promise<ConnectionPoolStats> {
  const response = await get<ConnectionPoolStats>(`/resources/pools/${poolId}`);
  if (!response.data) {
    throw new Error('Connection pool not found');
  }
  return response.data;
}

/**
 * Create a new connection pool
 */
export async function createPool(config: CreatePoolRequest): Promise<ConnectionPoolStats> {
  const response = await post<ConnectionPoolStats>('/resources/pools', config);
  if (!response.data) {
    throw new Error('Failed to create connection pool');
  }
  return response.data;
}

/**
 * Update an existing connection pool
 */
export async function updatePool(
  poolId: string,
  config: UpdatePoolRequest
): Promise<ConnectionPoolStats> {
  const response = await put<ConnectionPoolStats>(`/resources/pools/${poolId}`, config);
  if (!response.data) {
    throw new Error('Failed to update connection pool');
  }
  return response.data;
}

/**
 * Delete a connection pool
 */
export async function deletePool(poolId: string): Promise<void> {
  await del(`/resources/pools/${poolId}`);
}

/**
 * Get statistics for a specific pool
 */
export async function getPoolStats(poolId: string): Promise<ConnectionPoolStats> {
  const response = await get<ConnectionPoolStats>(`/resources/pools/${poolId}/stats`);
  if (!response.data) {
    throw new Error('Pool statistics not found');
  }
  return response.data;
}

/**
 * Drain a connection pool (gracefully close idle connections)
 */
export async function drainPool(poolId: string): Promise<void> {
  await post(`/resources/pools/${poolId}/drain`);
}

/**
 * Refill a drained connection pool
 */
export async function refillPool(poolId: string): Promise<void> {
  await post(`/resources/pools/${poolId}/refill`);
}

/**
 * Get pool health status
 */
export async function getPoolHealth(poolId: string): Promise<{
  healthy: boolean;
  utilization: number;
  issues: string[];
}> {
  const response = await get<{
    healthy: boolean;
    utilization: number;
    issues: string[];
  }>(`/resources/pools/${poolId}/health`);
  if (!response.data) {
    throw new Error('Pool health data not found');
  }
  return response.data;
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Calculate resource utilization percentage
 */
export function calculateUtilization(used: number, total: number): number {
  if (total === 0) return 0;
  return Math.min((used / total) * 100, 100);
}

/**
 * Format bytes to human-readable format
 */
export function formatBytes(bytes: number, decimals: number = 2): string {
  if (bytes === 0) return '0 Bytes';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

/**
 * Get health status based on utilization
 */
export function getHealthStatus(
  utilization: number
): 'healthy' | 'degraded' | 'unhealthy' | 'critical' {
  if (utilization >= 95) return 'critical';
  if (utilization >= 85) return 'unhealthy';
  if (utilization >= 70) return 'degraded';
  return 'healthy';
}

/**
 * Validate resource group configuration
 */
export function validateResourceGroupConfig(
  config: CreateResourceGroupRequest | UpdateResourceGroupRequest
): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  if ('cpuLimit' in config && config.cpuLimit !== undefined) {
    if (config.cpuLimit < 1 || config.cpuLimit > 100) {
      errors.push('CPU limit must be between 1 and 100%');
    }
  }

  if ('memoryLimit' in config && config.memoryLimit !== undefined) {
    if (config.memoryLimit < 1024 * 1024 * 1024) {
      errors.push('Memory limit must be at least 1 GB');
    }
  }

  if ('maxConnections' in config && config.maxConnections !== undefined) {
    if (config.maxConnections < 1) {
      errors.push('Max connections must be at least 1');
    }
  }

  if ('priority' in config && config.priority !== undefined) {
    if (config.priority < 1 || config.priority > 10) {
      errors.push('Priority must be between 1 and 10');
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Validate pool configuration
 */
export function validatePoolConfig(
  config: CreatePoolRequest | UpdatePoolRequest
): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  if ('minConnections' in config && config.minConnections !== undefined) {
    if (config.minConnections < 1) {
      errors.push('Minimum connections must be at least 1');
    }
  }

  if ('maxConnections' in config && config.maxConnections !== undefined) {
    if (config.maxConnections !== undefined && config.minConnections !== undefined) {
      if (config.maxConnections < config.minConnections) {
        errors.push('Maximum connections must be greater than or equal to minimum');
      }
    }
  }

  if ('connectionTimeout' in config && config.connectionTimeout !== undefined) {
    if (config.connectionTimeout < 1000) {
      errors.push('Connection timeout must be at least 1000ms');
    }
  }

  if ('idleTimeout' in config && config.idleTimeout !== undefined) {
    if (config.idleTimeout < 10000) {
      errors.push('Idle timeout must be at least 10000ms');
    }
  }

  if ('validationInterval' in config && config.validationInterval !== undefined) {
    if (config.validationInterval < 10000) {
      errors.push('Validation interval must be at least 10000ms');
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}
