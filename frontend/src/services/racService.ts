/**
 * RAC (Real Application Clusters) Service
 * Enterprise clustering and cache fusion management
 */

import api from './api';
import type { ApiResponse } from '../types/api';

export interface ClusterNode {
  id: string;
  name: string;
  host: string;
  port: number;
  status: 'active' | 'inactive' | 'failed' | 'joining';
  role: 'master' | 'replica';
  health: number;
  load: number;
  memory_used: number;
  memory_total: number;
  cpu_usage: number;
  connections: number;
  last_heartbeat: string;
  version: string;
}

export interface ClusterStatus {
  cluster_id: string;
  status: string;
  total_nodes: number;
  active_nodes: number;
  failed_nodes: number;
  nodes: ClusterNode[];
  cache_fusion_enabled: boolean;
  grd_enabled: boolean;
  interconnect_type: string;
}

export interface CacheFusionStats {
  total_transfers: number;
  successful_transfers: number;
  failed_transfers: number;
  avg_transfer_time_ms: number;
  total_bytes_transferred: number;
  cache_hit_rate: number;
  fusion_rate: number;
  local_reads: number;
  remote_reads: number;
  block_pings: number;
  cr_blocks_received: number;
  current_blocks_received: number;
}

export interface GrdResource {
  resource_id: string;
  resource_name: string;
  resource_type: string;
  master_node: string;
  lock_count: number;
  conversion_count: number;
  average_wait_time_ms: number;
  status: string;
}

export interface InterconnectStats {
  node_from: string;
  node_to: string;
  packets_sent: number;
  packets_received: number;
  bytes_sent: number;
  bytes_received: number;
  latency_avg_ms: number;
  latency_p99_ms: number;
  errors: number;
  bandwidth_mbps: number;
}

export interface RemasterRequest {
  target_node?: string;
  resources?: string[];
  strategy: 'immediate' | 'graceful';
}

class RacService {
  /**
   * Get cluster status
   */
  async getClusterStatus(): Promise<ApiResponse<ClusterStatus>> {
    return api.get('/api/rac/cluster/status');
  }

  /**
   * Get cluster nodes
   */
  async getClusterNodes(): Promise<ApiResponse<ClusterNode[]>> {
    return api.get('/api/rac/cluster/nodes');
  }

  /**
   * Get cluster statistics
   */
  async getClusterStats(): Promise<ApiResponse<any>> {
    return api.get('/api/rac/cluster/stats');
  }

  /**
   * Trigger cluster rebalance
   */
  async triggerRebalance(): Promise<ApiResponse<{ message: string; job_id: string }>> {
    return api.post('/api/rac/cluster/rebalance', {});
  }

  /**
   * Get cache fusion status
   */
  async getCacheFusionStatus(): Promise<ApiResponse<any>> {
    return api.get('/api/rac/cache-fusion/status');
  }

  /**
   * Get cache fusion statistics
   */
  async getCacheFusionStats(): Promise<ApiResponse<CacheFusionStats>> {
    return api.get('/api/rac/cache-fusion/stats');
  }

  /**
   * Get cache fusion block transfers
   */
  async getCacheFusionTransfers(): Promise<ApiResponse<any[]>> {
    return api.get('/api/rac/cache-fusion/transfers');
  }

  /**
   * Flush cache fusion
   */
  async flushCacheFusion(request: { node_id?: string; force: boolean }): Promise<ApiResponse<{ message: string }>> {
    return api.post('/api/rac/cache-fusion/flush', request);
  }

  /**
   * Get GRD topology
   */
  async getGrdTopology(): Promise<ApiResponse<any>> {
    return api.get('/api/rac/grd/topology');
  }

  /**
   * Get GRD resources
   */
  async getGrdResources(): Promise<ApiResponse<GrdResource[]>> {
    return api.get('/api/rac/grd/resources');
  }

  /**
   * Trigger GRD remastering
   */
  async triggerRemaster(request: RemasterRequest): Promise<ApiResponse<{ message: string; job_id: string }>> {
    return api.post('/api/rac/grd/remaster', request);
  }

  /**
   * Get interconnect status
   */
  async getInterconnectStatus(): Promise<ApiResponse<any>> {
    return api.get('/api/rac/interconnect/status');
  }

  /**
   * Get interconnect statistics
   */
  async getInterconnectStats(): Promise<ApiResponse<InterconnectStats[]>> {
    return api.get('/api/rac/interconnect/stats');
  }
}

export default new RacService();
