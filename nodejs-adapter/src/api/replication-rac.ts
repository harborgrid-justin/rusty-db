/**
 * RustyDB Replication & RAC (Real Application Clusters) API Adapter
 *
 * Provides comprehensive TypeScript interfaces and methods for:
 * - Replication configuration and management
 * - Replication slots (logical and physical)
 * - Replication conflict detection and resolution
 * - RAC cluster management
 * - Cache Fusion operations
 * - Global Resource Directory (GRD)
 * - Cluster interconnect monitoring
 * - Parallel query execution
 *
 * @module replication-rac
 */

import axios, { AxiosInstance, AxiosRequestConfig } from 'axios';

// ============================================================================
// Replication Types
// ============================================================================

/**
 * Replication configuration
 */
export interface ReplicationConfig {
  /** Replication mode: synchronous, asynchronous, or semi_synchronous */
  mode: 'synchronous' | 'asynchronous' | 'semi_synchronous';
  /** List of standby node addresses */
  standby_nodes: string[];
  /** Replication timeout in seconds */
  replication_timeout_secs?: number;
  /** Maximum number of WAL sender processes */
  max_wal_senders?: number;
  /** Number of WAL segments to keep */
  wal_keep_segments?: number;
  /** Enable WAL archiving */
  archive_mode?: boolean;
  /** Archive command to execute */
  archive_command?: string;
}

/**
 * Replication slot information
 */
export interface ReplicationSlot {
  /** Unique slot name */
  slot_name: string;
  /** Plugin name (logical) or 'physical' */
  plugin: string;
  /** Slot type: logical or physical */
  slot_type: 'logical' | 'physical';
  /** Database name (for logical slots) */
  database?: string;
  /** Whether the slot is currently active */
  active: boolean;
  /** Restart LSN (Log Sequence Number) */
  restart_lsn?: string;
  /** Confirmed flush LSN */
  confirmed_flush_lsn?: string;
  /** WAL status */
  wal_status: string;
  /** Catalog transaction ID minimum */
  catalog_xmin?: number;
  /** Restart delay in bytes */
  restart_delay?: number;
}

/**
 * Create replication slot request
 */
export interface CreateSlotRequest {
  /** Unique slot name */
  slot_name: string;
  /** Slot type: logical or physical */
  slot_type: 'logical' | 'physical';
  /** Plugin for logical slots (e.g., 'pgoutput') */
  plugin?: string;
  /** Whether this is a temporary slot */
  temporary?: boolean;
}

/**
 * Replication conflict information
 */
export interface ReplicationConflict {
  /** Unique conflict identifier */
  conflict_id: string;
  /** Database name */
  database: string;
  /** Table name where conflict occurred */
  table_name: string;
  /** Conflict type */
  conflict_type: 'update_conflict' | 'delete_conflict' | 'uniqueness_violation';
  /** Origin node ID */
  origin_node: string;
  /** Target node ID */
  target_node: string;
  /** When the conflict was detected (Unix timestamp) */
  detected_at: number;
  /** Local data version */
  local_data: any;
  /** Remote data version */
  remote_data: any;
  /** Resolution strategy applied */
  resolution_strategy?: string;
  /** Whether the conflict has been resolved */
  resolved: boolean;
  /** When the conflict was resolved (Unix timestamp) */
  resolved_at?: number;
}

/**
 * Resolve conflict request
 */
export interface ResolveConflictRequest {
  /** Conflict ID to resolve */
  conflict_id: string;
  /** Resolution strategy */
  strategy: 'use_local' | 'use_remote' | 'manual' | 'last_write_wins';
  /** Manual data for 'manual' strategy */
  manual_data?: any;
}

/**
 * Replication configuration response
 */
export interface ReplicationConfigResponse {
  success: boolean;
  message: string;
  config: ReplicationConfig;
}

/**
 * Slot list response
 */
export interface SlotListResponse {
  slots: ReplicationSlot[];
  total_count: number;
}

/**
 * Conflict list response
 */
export interface ConflictListResponse {
  conflicts: ReplicationConflict[];
  total_count: number;
  unresolved_count: number;
}

// ============================================================================
// RAC Cluster Types
// ============================================================================

/**
 * RAC cluster status
 */
export interface RACClusterStatus {
  /** Current cluster state */
  state: string;
  /** Whether cluster has quorum */
  has_quorum: boolean;
  /** Number of healthy nodes */
  healthy_nodes: number;
  /** Total nodes in cluster */
  total_nodes: number;
  /** Number of suspected nodes */
  suspected_nodes: number;
  /** Number of down nodes */
  down_nodes: number;
  /** Number of active recoveries */
  active_recoveries: number;
  /** Overall health status */
  is_healthy: boolean;
  /** Timestamp */
  timestamp: number;
}

/**
 * Cluster node information
 */
export interface ClusterNode {
  /** Node identifier */
  node_id: string;
  /** Network address */
  address: string;
  /** Node role */
  role: string;
  /** Node status */
  status: string;
  /** CPU cores */
  cpu_cores: number;
  /** Total memory in GB */
  total_memory_gb: number;
  /** Available memory in GB */
  available_memory_gb: number;
  /** Active services */
  services: string[];
  /** Node priority */
  priority: number;
}

/**
 * Cluster statistics
 */
export interface ClusterStats {
  /** Total nodes */
  total_nodes: number;
  /** Active nodes */
  active_nodes: number;
  /** Failed nodes */
  failed_nodes: number;
  /** Cluster uptime in seconds */
  uptime_seconds: number;
  /** Total transactions processed */
  total_transactions: number;
  /** Total queries executed */
  total_queries: number;
  /** Cache Fusion statistics */
  cache_fusion: CacheFusionStats;
  /** GRD statistics */
  grd: GRDStats;
  /** Interconnect statistics */
  interconnect: InterconnectStats;
}

// ============================================================================
// Cache Fusion Types
// ============================================================================

/**
 * Cache Fusion statistics
 */
export interface CacheFusionStats {
  /** Total block requests */
  total_requests: number;
  /** Successful block grants */
  successful_grants: number;
  /** Failed requests */
  failed_requests: number;
  /** Cache hits */
  cache_hits: number;
  /** Cache misses */
  cache_misses: number;
  /** Total bytes transferred */
  bytes_transferred: number;
  /** Average transfer latency in microseconds */
  avg_transfer_latency_us: number;
  /** Number of write-backs */
  write_backs: number;
  /** Number of downgrades */
  downgrades: number;
  /** Hit rate percentage */
  hit_rate_percent: number;
}

/**
 * Cache Fusion status
 */
export interface CacheFusionStatus {
  /** Is Cache Fusion enabled */
  enabled: boolean;
  /** Zero-copy transfers enabled */
  zero_copy_enabled: boolean;
  /** Prefetching enabled */
  prefetch_enabled: boolean;
  /** Active block transfers */
  active_transfers: number;
  /** Pending requests */
  pending_requests: number;
  /** Local cache size in blocks */
  local_cache_blocks: number;
  /** Current statistics */
  statistics: CacheFusionStats;
}

/**
 * Block transfer information
 */
export interface BlockTransferInfo {
  /** Resource identifier */
  resource_id: string;
  /** Source node */
  source_node: string;
  /** Target node */
  target_node: string;
  /** Block mode */
  block_mode: string;
  /** Transfer size in bytes */
  size_bytes: number;
  /** Transfer latency in microseconds */
  latency_us: number;
  /** Timestamp */
  timestamp: number;
}

/**
 * Cache flush request
 */
export interface CacheFlushRequest {
  /** Flush dirty blocks to disk */
  flush_dirty?: boolean;
  /** Invalidate clean blocks */
  invalidate_clean?: boolean;
}

// ============================================================================
// Global Resource Directory (GRD) Types
// ============================================================================

/**
 * GRD statistics
 */
export interface GRDStats {
  /** Total resources tracked */
  total_resources: number;
  /** Resources per master node */
  resources_per_master: Record<string, number>;
  /** Total remaster operations */
  total_remasters: number;
  /** Average remaster latency in milliseconds */
  avg_remaster_latency_ms: number;
  /** Affinity score updates */
  affinity_updates: number;
  /** Load balance operations */
  load_balances: number;
}

/**
 * GRD topology
 */
export interface GRDTopology {
  /** Cluster members */
  members: string[];
  /** Resource masters mapping */
  resource_masters: Record<string, string>;
  /** Hash ring distribution buckets */
  hash_ring_buckets: number;
  /** Load distribution per node */
  load_distribution: Record<string, number>;
}

/**
 * GRD resource entry
 */
export interface GRDResource {
  /** Resource identifier */
  resource_id: string;
  /** File ID */
  file_id: number;
  /** Block number */
  block_number: number;
  /** Resource class */
  resource_class: string;
  /** Master instance */
  master_instance: string;
  /** Shadow master */
  shadow_master?: string;
  /** Current mode */
  master_mode: string;
  /** Total accesses */
  total_accesses: number;
  /** Remote accesses */
  remote_accesses: number;
  /** Access pattern */
  access_pattern: string;
}

/**
 * Remaster request
 */
export interface RemasterRequest {
  /** Force remaster even if not needed */
  force?: boolean;
  /** Target node for specific resource */
  target_node?: string;
}

// ============================================================================
// Interconnect Types
// ============================================================================

/**
 * Interconnect statistics
 */
export interface InterconnectStats {
  /** Total messages sent */
  messages_sent: number;
  /** Total messages received */
  messages_received: number;
  /** Total bytes sent */
  bytes_sent: number;
  /** Total bytes received */
  bytes_received: number;
  /** Average message latency in microseconds */
  avg_message_latency_us: number;
  /** Failed message sends */
  failed_sends: number;
  /** Heartbeat failures */
  heartbeat_failures: number;
  /** Average throughput in MB/s */
  avg_throughput_mbps: number;
}

/**
 * Interconnect status
 */
export interface InterconnectStatus {
  /** Local node ID */
  local_node: string;
  /** Listen address */
  listen_address: string;
  /** Total connected nodes */
  connected_nodes: number;
  /** Healthy nodes */
  healthy_nodes: string[];
  /** Suspected nodes */
  suspected_nodes: string[];
  /** Down nodes */
  down_nodes: string[];
  /** Active connections */
  active_connections: number;
  /** Is interconnect running */
  is_running: boolean;
}

// ============================================================================
// Cluster Management Types
// ============================================================================

/**
 * Cluster node information
 */
export interface ClusterNodeInfo {
  node_id: string;
  address: string;
  role: string;
  status: string;
  version: string;
  uptime_seconds: number;
  last_heartbeat: number;
}

/**
 * Add node request
 */
export interface AddNodeRequest {
  node_id: string;
  address: string;
  role?: string;
}

/**
 * Cluster topology
 */
export interface ClusterTopology {
  cluster_id: string;
  nodes: ClusterNodeInfo[];
  leader_node?: string;
  quorum_size: number;
  total_nodes: number;
}

/**
 * Failover request
 */
export interface FailoverRequest {
  target_node?: string;
  force?: boolean;
}

/**
 * Replication status
 */
export interface ReplicationStatus {
  primary_node: string;
  replicas: ReplicaStatus[];
  replication_lag_ms: number;
  sync_state: string;
}

/**
 * Replica status
 */
export interface ReplicaStatus {
  node_id: string;
  state: string;
  lag_bytes: number;
  lag_ms: number;
  last_sync: number;
}

// ============================================================================
// API Client Configuration
// ============================================================================

/**
 * Configuration options for the ReplicationRACClient
 */
export interface ReplicationRACClientConfig {
  /** Base URL of the RustyDB API */
  baseURL: string;
  /** API key for authentication */
  apiKey?: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Additional axios configuration */
  axiosConfig?: AxiosRequestConfig;
}

// ============================================================================
// Main Client Class
// ============================================================================

/**
 * RustyDB Replication & RAC API Client
 *
 * Provides methods for managing replication, RAC clusters, Cache Fusion,
 * Global Resource Directory, and cluster interconnects.
 */
export class ReplicationRACClient {
  private client: AxiosInstance;

  /**
   * Create a new ReplicationRACClient
   * @param config Client configuration
   */
  constructor(config: ReplicationRACClientConfig) {
    this.client = axios.create({
      baseURL: config.baseURL,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        ...(config.apiKey && { 'X-API-Key': config.apiKey }),
      },
      ...config.axiosConfig,
    });
  }

  // ==========================================================================
  // Replication Configuration Methods
  // ==========================================================================

  /**
   * Configure replication settings
   * @param config Replication configuration
   * @returns Configuration response
   */
  async configureReplication(config: ReplicationConfig): Promise<ReplicationConfigResponse> {
    const response = await this.client.post<ReplicationConfigResponse>(
      '/api/v1/replication/configure',
      config
    );
    return response.data;
  }

  /**
   * Get current replication configuration
   * @returns Current replication configuration
   */
  async getReplicationConfig(): Promise<ReplicationConfig> {
    const response = await this.client.get<ReplicationConfig>('/api/v1/replication/config');
    return response.data;
  }

  // ==========================================================================
  // Replication Slot Methods
  // ==========================================================================

  /**
   * List all replication slots
   * @returns List of replication slots
   */
  async listReplicationSlots(): Promise<SlotListResponse> {
    const response = await this.client.get<SlotListResponse>('/api/v1/replication/slots');
    return response.data;
  }

  /**
   * Create a new replication slot
   * @param request Slot creation request
   * @returns Created replication slot
   */
  async createReplicationSlot(request: CreateSlotRequest): Promise<ReplicationSlot> {
    const response = await this.client.post<ReplicationSlot>(
      '/api/v1/replication/slots',
      request
    );
    return response.data;
  }

  /**
   * Get a specific replication slot by name
   * @param name Slot name
   * @returns Replication slot information
   */
  async getReplicationSlot(name: string): Promise<ReplicationSlot> {
    const response = await this.client.get<ReplicationSlot>(
      `/api/v1/replication/slots/${encodeURIComponent(name)}`
    );
    return response.data;
  }

  /**
   * Delete a replication slot
   * @param name Slot name
   */
  async deleteReplicationSlot(name: string): Promise<void> {
    await this.client.delete(`/api/v1/replication/slots/${encodeURIComponent(name)}`);
  }

  // ==========================================================================
  // Replication Conflict Methods
  // ==========================================================================

  /**
   * Get all replication conflicts
   * @returns List of replication conflicts
   */
  async getReplicationConflicts(): Promise<ConflictListResponse> {
    const response = await this.client.get<ConflictListResponse>('/api/v1/replication/conflicts');
    return response.data;
  }

  /**
   * Resolve a replication conflict
   * @param request Conflict resolution request
   * @returns Resolution result
   */
  async resolveReplicationConflict(request: ResolveConflictRequest): Promise<any> {
    const response = await this.client.post(
      '/api/v1/replication/resolve-conflict',
      request
    );
    return response.data;
  }

  /**
   * Simulate a replication conflict (for testing)
   * @returns Simulated conflict
   */
  async simulateReplicationConflict(): Promise<ReplicationConflict> {
    const response = await this.client.post<ReplicationConflict>(
      '/api/v1/replication/conflicts/simulate'
    );
    return response.data;
  }

  // ==========================================================================
  // RAC Cluster Management Methods
  // ==========================================================================

  /**
   * Get RAC cluster status
   * @returns Cluster status
   */
  async getClusterStatus(): Promise<RACClusterStatus> {
    const response = await this.client.get<RACClusterStatus>('/api/v1/rac/cluster/status');
    return response.data;
  }

  /**
   * Get all cluster nodes
   * @returns List of cluster nodes
   */
  async getClusterNodes(): Promise<ClusterNode[]> {
    const response = await this.client.get<ClusterNode[]>('/api/v1/rac/cluster/nodes');
    return response.data;
  }

  /**
   * Get cluster statistics
   * @returns Cluster statistics
   */
  async getClusterStats(): Promise<ClusterStats> {
    const response = await this.client.get<ClusterStats>('/api/v1/rac/cluster/stats');
    return response.data;
  }

  /**
   * Trigger cluster rebalance
   * @returns Rebalance result
   */
  async triggerClusterRebalance(): Promise<any> {
    const response = await this.client.post('/api/v1/rac/cluster/rebalance');
    return response.data;
  }

  // ==========================================================================
  // Cache Fusion Methods
  // ==========================================================================

  /**
   * Get Cache Fusion status
   * @returns Cache Fusion status
   */
  async getCacheFusionStatus(): Promise<CacheFusionStatus> {
    const response = await this.client.get<CacheFusionStatus>(
      '/api/v1/rac/cache-fusion/status'
    );
    return response.data;
  }

  /**
   * Get Cache Fusion statistics
   * @returns Cache Fusion statistics
   */
  async getCacheFusionStats(): Promise<CacheFusionStats> {
    const response = await this.client.get<CacheFusionStats>(
      '/api/v1/rac/cache-fusion/stats'
    );
    return response.data;
  }

  /**
   * Get recent Cache Fusion transfers
   * @returns List of recent block transfers
   */
  async getCacheFusionTransfers(): Promise<BlockTransferInfo[]> {
    const response = await this.client.get<BlockTransferInfo[]>(
      '/api/v1/rac/cache-fusion/transfers'
    );
    return response.data;
  }

  /**
   * Flush Cache Fusion cache
   * @param request Cache flush options
   * @returns Flush result
   */
  async flushCacheFusion(request: CacheFlushRequest): Promise<any> {
    const response = await this.client.post('/api/v1/rac/cache-fusion/flush', request);
    return response.data;
  }

  // ==========================================================================
  // Global Resource Directory (GRD) Methods
  // ==========================================================================

  /**
   * Get GRD topology
   * @returns GRD topology information
   */
  async getGRDTopology(): Promise<GRDTopology> {
    const response = await this.client.get<GRDTopology>('/api/v1/rac/grd/topology');
    return response.data;
  }

  /**
   * Get GRD resources
   * @returns List of GRD resources
   */
  async getGRDResources(): Promise<GRDResource[]> {
    const response = await this.client.get<GRDResource[]>('/api/v1/rac/grd/resources');
    return response.data;
  }

  /**
   * Trigger GRD remastering
   * @param request Remaster options
   * @returns Remaster result
   */
  async triggerGRDRemaster(request: RemasterRequest): Promise<any> {
    const response = await this.client.post('/api/v1/rac/grd/remaster', request);
    return response.data;
  }

  // ==========================================================================
  // Interconnect Methods
  // ==========================================================================

  /**
   * Get interconnect status
   * @returns Interconnect status
   */
  async getInterconnectStatus(): Promise<InterconnectStatus> {
    const response = await this.client.get<InterconnectStatus>(
      '/api/v1/rac/interconnect/status'
    );
    return response.data;
  }

  /**
   * Get interconnect statistics
   * @returns Interconnect statistics
   */
  async getInterconnectStats(): Promise<InterconnectStats> {
    const response = await this.client.get<InterconnectStats>(
      '/api/v1/rac/interconnect/stats'
    );
    return response.data;
  }

  // ==========================================================================
  // Cluster Management Methods (from cluster.rs)
  // ==========================================================================

  /**
   * Get basic cluster nodes list
   * @returns List of cluster nodes
   */
  async getBasicClusterNodes(): Promise<ClusterNodeInfo[]> {
    const response = await this.client.get<ClusterNodeInfo[]>('/api/v1/cluster/nodes');
    return response.data;
  }

  /**
   * Add a new cluster node
   * @param request Add node request
   * @returns Added node information
   */
  async addClusterNode(request: AddNodeRequest): Promise<ClusterNodeInfo> {
    const response = await this.client.post<ClusterNodeInfo>('/api/v1/cluster/nodes', request);
    return response.data;
  }

  /**
   * Get cluster node by ID
   * @param nodeId Node identifier
   * @returns Node information
   */
  async getClusterNode(nodeId: string): Promise<ClusterNodeInfo> {
    const response = await this.client.get<ClusterNodeInfo>(
      `/api/v1/cluster/nodes/${encodeURIComponent(nodeId)}`
    );
    return response.data;
  }

  /**
   * Remove a cluster node
   * @param nodeId Node identifier
   */
  async removeClusterNode(nodeId: string): Promise<void> {
    await this.client.delete(`/api/v1/cluster/nodes/${encodeURIComponent(nodeId)}`);
  }

  /**
   * Get cluster topology
   * @returns Cluster topology
   */
  async getClusterTopology(): Promise<ClusterTopology> {
    const response = await this.client.get<ClusterTopology>('/api/v1/cluster/topology');
    return response.data;
  }

  /**
   * Trigger manual failover
   * @param request Failover options
   */
  async triggerFailover(request: FailoverRequest): Promise<void> {
    await this.client.post('/api/v1/cluster/failover', request);
  }

  /**
   * Get replication status
   * @returns Replication status
   */
  async getBasicReplicationStatus(): Promise<ReplicationStatus> {
    const response = await this.client.get<ReplicationStatus>('/api/v1/cluster/replication');
    return response.data;
  }

  /**
   * Get cluster configuration
   * @returns Cluster configuration
   */
  async getClusterConfig(): Promise<Record<string, any>> {
    const response = await this.client.get<Record<string, any>>('/api/v1/cluster/config');
    return response.data;
  }

  /**
   * Update cluster configuration
   * @param config New configuration settings
   */
  async updateClusterConfig(config: Record<string, any>): Promise<void> {
    await this.client.put('/api/v1/cluster/config', config);
  }

  // ==========================================================================
  // Parallel Query Execution Methods
  // ==========================================================================

  /**
   * Execute a parallel query across RAC nodes
   * Note: This is a placeholder for parallel query execution functionality
   * @param sql SQL query to execute in parallel
   * @param options Parallel execution options
   * @returns Query results
   */
  async executeParallelQuery(
    sql: string,
    options?: {
      parallelism?: number;
      nodes?: string[];
      timeout?: number;
    }
  ): Promise<any> {
    // This would typically integrate with the query execution API
    // and leverage RAC cluster capabilities for parallel execution
    const response = await this.client.post('/api/v1/query/execute', {
      sql,
      parallel: true,
      parallelism: options?.parallelism,
      nodes: options?.nodes,
      timeout: options?.timeout,
    });
    return response.data;
  }
}

// Export everything
export default ReplicationRACClient;
