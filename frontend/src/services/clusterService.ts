// ============================================================================
// Cluster & Replication Service
// ============================================================================

import { get, post, del, patch } from './api';
import type {
  ClusterTopology,
  ClusterNode,
  ReplicationStatus,
  FailoverEvent,
  ReplicationConfig,
  NodeMetrics,
  UUID,
  Timestamp,
  Duration,
} from '../types';

// ============================================================================
// Request/Response Types
// ============================================================================

export interface AddNodeRequest {
  name: string;
  host: string;
  port: number;
  region?: string;
  zone?: string;
  initialSync?: boolean;
}

export interface RemoveNodeRequest {
  force?: boolean;
  preserveData?: boolean;
}

export interface ReplicationLagMetric {
  timestamp: Timestamp;
  lag: Duration;
  bytesPerSecond: number;
  transactionsPerSecond: number;
}

export interface ReplicationConfigUpdate {
  mode?: 'synchronous' | 'asynchronous' | 'semi_synchronous';
  syncStandbyNames?: string[];
  maxWalSenders?: number;
  walKeepSegments?: number;
}

export interface FailoverRequest {
  targetNodeId: UUID;
  force?: boolean;
  timeout?: number;
}

export interface FailoverPreflightCheck {
  nodeId: UUID;
  nodeName: string;
  isHealthy: boolean;
  canBeLeader: boolean;
  estimatedDowntime: Duration;
  checks: {
    name: string;
    status: 'pass' | 'warning' | 'fail';
    message: string;
  }[];
  warnings: string[];
}

export interface NodeSyncProgress {
  nodeId: UUID;
  phase: 'connecting' | 'initial_sync' | 'streaming' | 'complete';
  progress: number;
  bytesTransferred: number;
  totalBytes: number;
  estimatedTimeRemaining?: Duration;
  currentLsn?: string;
  targetLsn?: string;
}

// ============================================================================
// Cluster Topology APIs
// ============================================================================

export async function getClusterTopology(): Promise<ClusterTopology> {
  const response = await get<ClusterTopology>('/cluster/topology');
  return response.data!;
}

export async function getNodes(): Promise<ClusterNode[]> {
  const response = await get<ClusterNode[]>('/cluster/nodes');
  return response.data!;
}

export async function getNodeStatus(nodeId: UUID): Promise<ClusterNode> {
  const response = await get<ClusterNode>(`/cluster/nodes/${nodeId}`);
  return response.data!;
}

export async function getNodeMetrics(
  nodeId: UUID,
  timeRange?: { start: Timestamp; end: Timestamp }
): Promise<NodeMetrics[]> {
  const params = timeRange
    ? `?start=${timeRange.start}&end=${timeRange.end}`
    : '';
  const response = await get<NodeMetrics[]>(
    `/cluster/nodes/${nodeId}/metrics${params}`
  );
  return response.data!;
}

export async function addNode(config: AddNodeRequest): Promise<ClusterNode> {
  const response = await post<ClusterNode>('/cluster/nodes', config);
  return response.data!;
}

export async function removeNode(
  nodeId: UUID,
  options?: RemoveNodeRequest
): Promise<void> {
  await del(`/cluster/nodes/${nodeId}`, {
    data: options,
  });
}

export async function promoteNode(nodeId: UUID): Promise<void> {
  await post(`/cluster/nodes/${nodeId}/promote`);
}

export async function demoteNode(nodeId: UUID): Promise<void> {
  await post(`/cluster/nodes/${nodeId}/demote`);
}

// ============================================================================
// Replication APIs
// ============================================================================

export async function getReplicationStatus(): Promise<ReplicationStatus[]> {
  const response = await get<ReplicationStatus[]>('/cluster/replication/status');
  return response.data!;
}

export async function getReplicationStatusByNode(
  nodeId: UUID
): Promise<ReplicationStatus> {
  const response = await get<ReplicationStatus>(
    `/cluster/replication/status/${nodeId}`
  );
  return response.data!;
}

export async function getReplicationLag(
  nodeId: UUID,
  timeRange?: { start: Timestamp; end: Timestamp }
): Promise<ReplicationLagMetric[]> {
  const params = timeRange
    ? `?start=${timeRange.start}&end=${timeRange.end}`
    : '';
  const response = await get<ReplicationLagMetric[]>(
    `/cluster/replication/lag/${nodeId}${params}`
  );
  return response.data!;
}

export async function getReplicationConfig(): Promise<ReplicationConfig> {
  const response = await get<ReplicationConfig>('/cluster/replication/config');
  return response.data!;
}

export async function updateReplicationConfig(
  config: ReplicationConfigUpdate
): Promise<ReplicationConfig> {
  const response = await patch<ReplicationConfig>(
    '/cluster/replication/config',
    config
  );
  return response.data!;
}

export async function pauseReplication(nodeId: UUID): Promise<void> {
  await post(`/cluster/replication/${nodeId}/pause`);
}

export async function resumeReplication(nodeId: UUID): Promise<void> {
  await post(`/cluster/replication/${nodeId}/resume`);
}

export async function resyncNode(nodeId: UUID): Promise<void> {
  await post(`/cluster/replication/${nodeId}/resync`);
}

export async function getNodeSyncProgress(
  nodeId: UUID
): Promise<NodeSyncProgress> {
  const response = await get<NodeSyncProgress>(
    `/cluster/replication/${nodeId}/sync-progress`
  );
  return response.data!;
}

// ============================================================================
// Failover APIs
// ============================================================================

export async function getFailoverHistory(
  limit: number = 50
): Promise<FailoverEvent[]> {
  const response = await get<FailoverEvent[]>(
    `/cluster/failover/history?limit=${limit}`
  );
  return response.data!;
}

export async function getFailoverConfig(): Promise<{
  enabled: boolean;
  autoFailover: boolean;
  failoverTimeout: Duration;
  healthCheckInterval: Duration;
  minHealthyFollowers: number;
}> {
  const response = await get<{
    enabled: boolean;
    autoFailover: boolean;
    failoverTimeout: Duration;
    healthCheckInterval: Duration;
    minHealthyFollowers: number;
  }>('/cluster/failover/config');
  return response.data!;
}

export async function updateFailoverConfig(config: {
  autoFailover?: boolean;
  failoverTimeout?: Duration;
  healthCheckInterval?: Duration;
  minHealthyFollowers?: number;
}): Promise<void> {
  await patch('/cluster/failover/config', config);
}

export async function preflightFailoverCheck(
  targetNodeId: UUID
): Promise<FailoverPreflightCheck> {
  const response = await post<FailoverPreflightCheck>(
    '/cluster/failover/preflight',
    { targetNodeId }
  );
  return response.data!;
}

export async function triggerFailover(
  request: FailoverRequest
): Promise<FailoverEvent> {
  const response = await post<FailoverEvent>('/cluster/failover/trigger', request);
  return response.data!;
}

export async function cancelFailover(): Promise<void> {
  await post('/cluster/failover/cancel');
}

// ============================================================================
// Health & Diagnostics APIs
// ============================================================================

export async function getClusterHealth(): Promise<{
  healthy: boolean;
  leaderPresent: boolean;
  quorumSize: number;
  activeNodes: number;
  totalNodes: number;
  issues: string[];
}> {
  const response = await get<{
    healthy: boolean;
    leaderPresent: boolean;
    quorumSize: number;
    activeNodes: number;
    totalNodes: number;
    issues: string[];
  }>('/cluster/health');
  return response.data!;
}

export async function runClusterDiagnostics(): Promise<{
  timestamp: Timestamp;
  checks: {
    name: string;
    status: 'pass' | 'warning' | 'fail';
    message: string;
    details?: Record<string, unknown>;
  }[];
}> {
  const response = await post<{
    timestamp: Timestamp;
    checks: {
      name: string;
      status: 'pass' | 'warning' | 'fail';
      message: string;
      details?: Record<string, unknown>;
    }[];
  }>('/cluster/diagnostics');
  return response.data!;
}

// ============================================================================
// Cluster Configuration APIs
// ============================================================================

export async function getClusterConfig(): Promise<{
  name: string;
  quorumSize: number;
  electionTimeout: Duration;
  heartbeatInterval: Duration;
  replication: ReplicationConfig;
}> {
  const response = await get<{
    name: string;
    quorumSize: number;
    electionTimeout: Duration;
    heartbeatInterval: Duration;
    replication: ReplicationConfig;
  }>('/cluster/config');
  return response.data!;
}

export async function updateClusterConfig(config: {
  name?: string;
  electionTimeout?: Duration;
  heartbeatInterval?: Duration;
}): Promise<void> {
  await patch('/cluster/config', config);
}
