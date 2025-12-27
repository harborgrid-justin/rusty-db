import { get, post, put, del, buildQueryParams } from './api';
import type {
  UUID,
  Timestamp,
  Duration,
  ClusterNode,
  PaginationParams,
  PaginatedResponse,
} from '../types';

// ============================================================================
// Networking Service Types
// ============================================================================

export interface NetworkStatus {
  serverStatus: 'online' | 'offline' | 'degraded';
  listenAddress: string;
  listenPort: number;
  protocolVersion: string;
  uptime: Duration;
  activeConnections: number;
  maxConnections: number;
  totalConnectionsAccepted: number;
  totalConnectionsRejected: number;
  bytesReceived: number;
  bytesSent: number;
  packetsReceived: number;
  packetsSent: number;
  networkErrors: number;
  lastHeartbeat: Timestamp;
}

export interface NetworkConnection {
  id: UUID;
  remoteAddress: string;
  remotePort: number;
  localAddress: string;
  localPort: number;
  protocol: ProtocolType;
  state: ConnectionState;
  userId?: string;
  database?: string;
  applicationName?: string;
  establishedAt: Timestamp;
  lastActivity: Timestamp;
  bytesSent: number;
  bytesReceived: number;
  requestCount: number;
  errorCount: number;
  avgResponseTime: Duration;
  sslEnabled: boolean;
  sslVersion?: string;
  sslCipher?: string;
}

export type ProtocolType = 'postgresql' | 'mysql' | 'http' | 'grpc' | 'websocket' | 'custom';

export type ConnectionState =
  | 'established'
  | 'idle'
  | 'active'
  | 'closing'
  | 'closed'
  | 'error';

export interface ProtocolConfiguration {
  postgresql: PostgresqlProtocolConfig;
  mysql: MysqlProtocolConfig;
  http: HttpProtocolConfig;
  grpc: GrpcProtocolConfig;
  websocket: WebsocketProtocolConfig;
}

export interface PostgresqlProtocolConfig {
  enabled: boolean;
  port: number;
  maxConnections: number;
  sslMode: 'disable' | 'allow' | 'prefer' | 'require' | 'verify-ca' | 'verify-full';
  sslCert?: string;
  sslKey?: string;
  sslRootCert?: string;
  statementTimeout: Duration;
  idleInTransactionTimeout: Duration;
  tcpKeepAlive: boolean;
  tcpKeepAliveInterval?: Duration;
}

export interface MysqlProtocolConfig {
  enabled: boolean;
  port: number;
  maxConnections: number;
  useCompression: boolean;
  maxPacketSize: number;
  sslEnabled: boolean;
  sslCert?: string;
  sslKey?: string;
  sslCa?: string;
}

export interface HttpProtocolConfig {
  enabled: boolean;
  port: number;
  maxConnections: number;
  corsEnabled: boolean;
  corsOrigins: string[];
  requestTimeout: Duration;
  maxRequestSize: number;
  tlsEnabled: boolean;
  tlsCert?: string;
  tlsKey?: string;
  rateLimitEnabled: boolean;
  rateLimitRequestsPerMinute?: number;
}

export interface GrpcProtocolConfig {
  enabled: boolean;
  port: number;
  maxConnections: number;
  maxMessageSize: number;
  keepAliveTime: Duration;
  keepAliveTimeout: Duration;
  tlsEnabled: boolean;
  tlsCert?: string;
  tlsKey?: string;
}

export interface WebsocketProtocolConfig {
  enabled: boolean;
  port: number;
  maxConnections: number;
  pingInterval: Duration;
  pongTimeout: Duration;
  maxMessageSize: number;
  compressionEnabled: boolean;
}

export interface UpdateProtocolsRequest {
  postgresql?: Partial<PostgresqlProtocolConfig>;
  mysql?: Partial<MysqlProtocolConfig>;
  http?: Partial<HttpProtocolConfig>;
  grpc?: Partial<GrpcProtocolConfig>;
  websocket?: Partial<WebsocketProtocolConfig>;
}

export interface ClusterNodeExtended extends ClusterNode {
  cpuUsage: number;
  memoryUsage: number;
  diskUsage: number;
  networkLatency?: Duration;
  uptime: Duration;
  connections: number;
  queriesPerSecond: number;
  replicationLag?: Duration;
  tags?: Record<string, string>;
}

export interface AddNodeRequest {
  name: string;
  host: string;
  port: number;
  role?: 'follower' | 'observer';
  region?: string;
  zone?: string;
  tags?: Record<string, string>;
  autoStart?: boolean;
}

export interface NodeHealthCheck {
  nodeId: UUID;
  status: 'healthy' | 'degraded' | 'unreachable';
  checks: HealthCheck[];
  lastCheck: Timestamp;
  nextCheck?: Timestamp;
}

export interface HealthCheck {
  name: string;
  status: 'pass' | 'fail' | 'warn';
  message?: string;
  responseTime?: Duration;
  details?: Record<string, unknown>;
}

export interface LoadBalancerStats {
  strategy: LoadBalancingStrategy;
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  averageResponseTime: Duration;
  nodeDistribution: NodeDistribution[];
  activeBackends: number;
  totalBackends: number;
  circuitBreakerStatus: CircuitBreakerStatus;
}

export type LoadBalancingStrategy =
  | 'round_robin'
  | 'least_connections'
  | 'least_response_time'
  | 'weighted_round_robin'
  | 'ip_hash'
  | 'random';

export interface NodeDistribution {
  nodeId: UUID;
  nodeName: string;
  requestCount: number;
  requestPercent: number;
  activeConnections: number;
  averageResponseTime: Duration;
  errorRate: number;
}

export interface CircuitBreakerStatus {
  state: 'closed' | 'open' | 'half_open';
  failureCount: number;
  successCount: number;
  failureThreshold: number;
  timeout: Duration;
  lastStateChange: Timestamp;
  nextRetry?: Timestamp;
}

export interface ConnectionFilters extends Partial<PaginationParams> {
  state?: ConnectionState;
  protocol?: ProtocolType;
  userId?: string;
  database?: string;
  minDuration?: number;
}

export interface NodeFilters extends Partial<PaginationParams> {
  role?: 'leader' | 'follower' | 'candidate' | 'observer';
  status?: 'healthy' | 'degraded' | 'unreachable' | 'shutting_down' | 'failed';
  region?: string;
  zone?: string;
}

export interface NetworkMetrics {
  timestamp: Timestamp;
  connectionRate: number;
  throughput: {
    bytesPerSecond: number;
    requestsPerSecond: number;
  };
  latency: {
    p50: Duration;
    p95: Duration;
    p99: Duration;
    max: Duration;
  };
  errors: {
    connectionErrors: number;
    timeoutErrors: number;
    protocolErrors: number;
    total: number;
  };
}

// ============================================================================
// Networking Service
// ============================================================================

export const networkingService = {
  // ============================================================================
  // Network Status
  // ============================================================================

  /**
   * Get overall network status
   */
  async getNetworkStatus(): Promise<NetworkStatus> {
    const response = await get<NetworkStatus>('/network/status');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch network status');
    }

    return response.data;
  },

  /**
   * Get network metrics history
   */
  async getNetworkMetrics(
    startTime?: Timestamp,
    endTime?: Timestamp,
    interval?: number
  ): Promise<NetworkMetrics[]> {
    const params = buildQueryParams({
      startTime,
      endTime,
      interval,
    });

    const response = await get<NetworkMetrics[]>(`/network/metrics${params}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch network metrics');
    }

    return response.data;
  },

  // ============================================================================
  // Connections
  // ============================================================================

  /**
   * List all active network connections
   */
  async getConnections(
    filters?: ConnectionFilters
  ): Promise<PaginatedResponse<NetworkConnection>> {
    const queryParams = filters ? buildQueryParams(filters as Record<string, unknown>) : '';
    const response = await get<PaginatedResponse<NetworkConnection>>(
      `/network/connections${queryParams}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch connections');
    }

    return response.data;
  },

  /**
   * Get a single connection by ID
   */
  async getConnection(id: UUID): Promise<NetworkConnection> {
    const response = await get<NetworkConnection>(`/network/connections/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch connection');
    }

    return response.data;
  },

  /**
   * Terminate a network connection
   */
  async terminateConnection(id: UUID): Promise<{ success: boolean; message: string }> {
    const response = await del<{ success: boolean; message: string }>(
      `/network/connections/${id}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to terminate connection');
    }

    return response.data;
  },

  // ============================================================================
  // Protocol Configuration
  // ============================================================================

  /**
   * Get current protocol configuration
   */
  async getProtocols(): Promise<ProtocolConfiguration> {
    const response = await get<ProtocolConfiguration>('/network/protocols');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch protocol configuration');
    }

    return response.data;
  },

  /**
   * Update protocol configuration
   */
  async updateProtocols(config: UpdateProtocolsRequest): Promise<ProtocolConfiguration> {
    const response = await put<ProtocolConfiguration>('/network/protocols', config);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to update protocol configuration');
    }

    return response.data;
  },

  // ============================================================================
  // Cluster Nodes
  // ============================================================================

  /**
   * List all cluster nodes
   */
  async getClusterNodes(
    filters?: NodeFilters
  ): Promise<PaginatedResponse<ClusterNodeExtended>> {
    const queryParams = filters ? buildQueryParams(filters as Record<string, unknown>) : '';
    const response = await get<PaginatedResponse<ClusterNodeExtended>>(
      `/network/cluster/nodes${queryParams}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch cluster nodes');
    }

    return response.data;
  },

  /**
   * Get a single cluster node by ID
   */
  async getClusterNode(id: UUID): Promise<ClusterNodeExtended> {
    const response = await get<ClusterNodeExtended>(`/network/cluster/nodes/${id}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch cluster node');
    }

    return response.data;
  },

  /**
   * Add a new node to the cluster
   */
  async addClusterNode(request: AddNodeRequest): Promise<ClusterNodeExtended> {
    const response = await post<ClusterNodeExtended>('/network/cluster/nodes', request);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to add cluster node');
    }

    return response.data;
  },

  /**
   * Remove a node from the cluster
   */
  async removeClusterNode(id: UUID, force = false): Promise<{ success: boolean; message: string }> {
    const response = await del<{ success: boolean; message: string }>(
      `/network/cluster/nodes/${id}?force=${force}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to remove cluster node');
    }

    return response.data;
  },

  /**
   * Perform health check on a cluster node
   */
  async checkNodeHealth(id: UUID): Promise<NodeHealthCheck> {
    const response = await post<NodeHealthCheck>(`/network/cluster/nodes/${id}/health-check`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to check node health');
    }

    return response.data;
  },

  /**
   * Promote a follower node to leader
   */
  async promoteNode(id: UUID): Promise<ClusterNodeExtended> {
    const response = await post<ClusterNodeExtended>(`/network/cluster/nodes/${id}/promote`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to promote node');
    }

    return response.data;
  },

  /**
   * Demote a leader node to follower
   */
  async demoteNode(id: UUID): Promise<ClusterNodeExtended> {
    const response = await post<ClusterNodeExtended>(`/network/cluster/nodes/${id}/demote`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to demote node');
    }

    return response.data;
  },

  // ============================================================================
  // Load Balancer
  // ============================================================================

  /**
   * Get load balancer statistics
   */
  async getLoadBalancerStats(): Promise<LoadBalancerStats> {
    const response = await get<LoadBalancerStats>('/network/loadbalancer');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch load balancer stats');
    }

    return response.data;
  },

  /**
   * Update load balancing strategy
   */
  async updateLoadBalancingStrategy(
    strategy: LoadBalancingStrategy
  ): Promise<LoadBalancerStats> {
    const response = await put<LoadBalancerStats>('/network/loadbalancer/strategy', {
      strategy,
    });

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to update load balancing strategy');
    }

    return response.data;
  },

  /**
   * Reset circuit breaker for a specific node
   */
  async resetCircuitBreaker(nodeId: UUID): Promise<{ success: boolean; message: string }> {
    const response = await post<{ success: boolean; message: string }>(
      `/network/loadbalancer/circuit-breaker/${nodeId}/reset`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to reset circuit breaker');
    }

    return response.data;
  },

  /**
   * Get all circuit breaker statuses
   */
  async getCircuitBreakers(): Promise<
    Array<{
      name: string;
      state: 'closed' | 'open' | 'half_open';
      failure_count: number;
      success_count: number;
      last_failure: number | null;
      last_state_change: number;
      failure_threshold: number;
      timeout_secs: number;
    }>
  > {
    const response = await get<
      Array<{
        name: string;
        state: 'closed' | 'open' | 'half_open';
        failure_count: number;
        success_count: number;
        last_failure: number | null;
        last_state_change: number;
        failure_threshold: number;
        timeout_secs: number;
      }>
    >('/network/circuit-breakers');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch circuit breakers');
    }

    return response.data;
  },
};
