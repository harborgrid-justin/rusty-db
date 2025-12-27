// ============================================================================
// Cluster Hooks - Real-time cluster data management
// ============================================================================

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from '../contexts/WebSocketContext';
import { useEffect } from 'react';
import toast from 'react-hot-toast';
import * as clusterService from '../services/clusterService';
import type {
  ClusterTopology,
  ClusterNode,
  ReplicationStatus,
  FailoverEvent,
  UUID,
  Timestamp,
} from '../types';

// ============================================================================
// Query Keys
// ============================================================================

// ============================================================================
// WebSocket Event Types
// ============================================================================

interface NodeStatusEvent {
  nodeId: UUID;
  status: Partial<ClusterNode>;
}

interface NodeAddedEvent {
  id: UUID;
  name: string;
}

interface NodeRemovedEvent {
  id: UUID;
  name: string;
}

interface ReplicationStatusEvent {
  targetNode: UUID;
  status: Partial<ReplicationStatus>;
}

interface ReplicationLagEvent {
  nodeId: UUID;
  metric: clusterService.ReplicationLagMetric;
}

export const clusterKeys = {
  all: ['cluster'] as const,
  topology: () => [...clusterKeys.all, 'topology'] as const,
  nodes: () => [...clusterKeys.all, 'nodes'] as const,
  node: (id: UUID) => [...clusterKeys.nodes(), id] as const,
  nodeMetrics: (id: UUID, timeRange?: { start: Timestamp; end: Timestamp }) =>
    [...clusterKeys.node(id), 'metrics', timeRange] as const,
  replication: () => [...clusterKeys.all, 'replication'] as const,
  replicationStatus: () => [...clusterKeys.replication(), 'status'] as const,
  replicationLag: (nodeId: UUID, timeRange?: { start: Timestamp; end: Timestamp }) =>
    [...clusterKeys.replication(), 'lag', nodeId, timeRange] as const,
  replicationConfig: () => [...clusterKeys.replication(), 'config'] as const,
  failover: () => [...clusterKeys.all, 'failover'] as const,
  failoverHistory: () => [...clusterKeys.failover(), 'history'] as const,
  failoverConfig: () => [...clusterKeys.failover(), 'config'] as const,
  health: () => [...clusterKeys.all, 'health'] as const,
  config: () => [...clusterKeys.all, 'config'] as const,
};

// ============================================================================
// Cluster Topology Hooks
// ============================================================================

export function useClusterTopology() {
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  const query = useQuery({
    queryKey: clusterKeys.topology(),
    queryFn: clusterService.getClusterTopology,
    refetchInterval: 10000, // Refetch every 10 seconds
    staleTime: 5000,
  });

  // Subscribe to real-time topology updates
  useEffect(() => {
    const unsubscribe = subscribe('cluster:topology', (data) => {
      queryClient.setQueryData<ClusterTopology>(
        clusterKeys.topology(),
        data as ClusterTopology
      );
    });

    return unsubscribe;
  }, [subscribe, queryClient]);

  return query;
}

export function useNodes() {
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  const query = useQuery({
    queryKey: clusterKeys.nodes(),
    queryFn: clusterService.getNodes,
    refetchInterval: 5000,
    staleTime: 3000,
  });

  // Subscribe to real-time node updates
  useEffect(() => {
    const unsubscribeNodeStatus = subscribe('cluster:node:status', (data: unknown) => {
      const { nodeId, status } = data as NodeStatusEvent;

      // Update the specific node in the nodes list
      queryClient.setQueryData<ClusterNode[]>(clusterKeys.nodes(), (old) => {
        if (!old) return old;
        return old.map((node) =>
          node.id === nodeId ? { ...node, ...status } : node
        );
      });

      // Also update the individual node query if it exists
      queryClient.setQueryData<ClusterNode>(
        clusterKeys.node(nodeId),
        (old) => (old ? { ...old, ...status } : old)
      );
    });

    const unsubscribeNodeAdded = subscribe('cluster:node:added', (data: unknown) => {
      const nodeData = data as NodeAddedEvent;
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      toast.success(`Node ${nodeData.name} added to cluster`);
    });

    const unsubscribeNodeRemoved = subscribe('cluster:node:removed', (data: unknown) => {
      const nodeData = data as NodeRemovedEvent;
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      toast.success(`Node ${nodeData.name} removed from cluster`);
    });

    return () => {
      unsubscribeNodeStatus();
      unsubscribeNodeAdded();
      unsubscribeNodeRemoved();
    };
  }, [subscribe, queryClient]);

  return query;
}

export function useNodeStatus(nodeId: UUID) {
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  const query = useQuery({
    queryKey: clusterKeys.node(nodeId),
    queryFn: () => clusterService.getNodeStatus(nodeId),
    refetchInterval: 5000,
    staleTime: 3000,
    enabled: !!nodeId,
  });

  // Subscribe to real-time updates for this specific node
  useEffect(() => {
    if (!nodeId) return;

    const unsubscribe = subscribe('cluster:node:status', (data: unknown) => {
      const statusEvent = data as NodeStatusEvent;
      if (statusEvent.nodeId === nodeId) {
        queryClient.setQueryData<ClusterNode>(
          clusterKeys.node(nodeId),
          (old) => (old ? { ...old, ...statusEvent.status } : old)
        );
      }
    });

    return unsubscribe;
  }, [nodeId, subscribe, queryClient]);

  return query;
}

export function useNodeMetrics(
  nodeId: UUID,
  timeRange?: { start: Timestamp; end: Timestamp }
) {
  return useQuery({
    queryKey: clusterKeys.nodeMetrics(nodeId, timeRange),
    queryFn: () => clusterService.getNodeMetrics(nodeId, timeRange),
    refetchInterval: 10000,
    staleTime: 5000,
    enabled: !!nodeId,
  });
}

// ============================================================================
// Node Management Mutations
// ============================================================================

export function useAddNode() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.addNode,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      toast.success('Node added successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to add node: ${error.message}`);
    },
  });
}

export function useRemoveNode() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ nodeId, options }: {
      nodeId: UUID;
      options?: clusterService.RemoveNodeRequest;
    }) => clusterService.removeNode(nodeId, options),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      toast.success('Node removed successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to remove node: ${error.message}`);
    },
  });
}

export function usePromoteNode() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.promoteNode,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      toast.success('Node promoted successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to promote node: ${error.message}`);
    },
  });
}

export function useDemoteNode() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.demoteNode,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      toast.success('Node demoted successfully');
    },
    onError: (error: Error) => {
      toast.error(`Failed to demote node: ${error.message}`);
    },
  });
}

// ============================================================================
// Replication Hooks
// ============================================================================

export function useReplicationStatus() {
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  const query = useQuery({
    queryKey: clusterKeys.replicationStatus(),
    queryFn: clusterService.getReplicationStatus,
    refetchInterval: 5000,
    staleTime: 3000,
  });

  // Subscribe to real-time replication updates
  useEffect(() => {
    const unsubscribe = subscribe('cluster:replication:status', (data) => {
      queryClient.setQueryData<ReplicationStatus[]>(
        clusterKeys.replicationStatus(),
        data as ReplicationStatus[]
      );
    });

    return unsubscribe;
  }, [subscribe, queryClient]);

  return query;
}

export function useReplicationStatusByNode(nodeId: UUID) {
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  const query = useQuery({
    queryKey: [...clusterKeys.replicationStatus(), nodeId],
    queryFn: () => clusterService.getReplicationStatusByNode(nodeId),
    refetchInterval: 5000,
    staleTime: 3000,
    enabled: !!nodeId,
  });

  // Subscribe to real-time updates for this node's replication
  useEffect(() => {
    if (!nodeId) return;

    const unsubscribe = subscribe('cluster:replication:status', (data: unknown) => {
      const statusEvent = data as ReplicationStatusEvent;
      if (statusEvent.targetNode === nodeId) {
        queryClient.setQueryData<ReplicationStatus>(
          [...clusterKeys.replicationStatus(), nodeId],
          statusEvent as unknown as ReplicationStatus
        );
      }
    });

    return unsubscribe;
  }, [nodeId, subscribe, queryClient]);

  return query;
}

export function useReplicationLag(
  nodeId: UUID,
  timeRange?: { start: Timestamp; end: Timestamp }
) {
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  const query = useQuery({
    queryKey: clusterKeys.replicationLag(nodeId, timeRange),
    queryFn: () => clusterService.getReplicationLag(nodeId, timeRange),
    refetchInterval: 5000,
    staleTime: 3000,
    enabled: !!nodeId,
  });

  // Subscribe to real-time lag updates
  useEffect(() => {
    if (!nodeId) return;

    const unsubscribe = subscribe('cluster:replication:lag', (data: unknown) => {
      const lagEvent = data as ReplicationLagEvent;
      if (lagEvent.nodeId === nodeId) {
        // Append new lag metric to the existing data
        queryClient.setQueryData<clusterService.ReplicationLagMetric[]>(
          clusterKeys.replicationLag(nodeId, timeRange),
          (old) => {
            if (!old) return [lagEvent.metric];
            return [...old, lagEvent.metric].slice(-100); // Keep last 100 data points
          }
        );
      }
    });

    return unsubscribe;
  }, [nodeId, timeRange, subscribe, queryClient]);

  return query;
}

export function useReplicationConfig() {
  return useQuery({
    queryKey: clusterKeys.replicationConfig(),
    queryFn: clusterService.getReplicationConfig,
    staleTime: 60000, // Config changes less frequently
  });
}

export function useUpdateReplicationConfig() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.updateReplicationConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.replicationConfig() });
      toast.success('Replication configuration updated');
    },
    onError: (error: Error) => {
      toast.error(`Failed to update replication config: ${error.message}`);
    },
  });
}

export function usePauseReplication() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.pauseReplication,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.replicationStatus() });
      toast.success('Replication paused');
    },
    onError: (error: Error) => {
      toast.error(`Failed to pause replication: ${error.message}`);
    },
  });
}

export function useResumeReplication() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.resumeReplication,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.replicationStatus() });
      toast.success('Replication resumed');
    },
    onError: (error: Error) => {
      toast.error(`Failed to resume replication: ${error.message}`);
    },
  });
}

export function useResyncNode() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.resyncNode,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.replicationStatus() });
      toast.success('Node resync started');
    },
    onError: (error: Error) => {
      toast.error(`Failed to start resync: ${error.message}`);
    },
  });
}

export function useNodeSyncProgress(nodeId: UUID) {
  return useQuery({
    queryKey: [...clusterKeys.replication(), 'sync-progress', nodeId],
    queryFn: () => clusterService.getNodeSyncProgress(nodeId),
    refetchInterval: 2000, // Check progress frequently
    staleTime: 1000,
    enabled: !!nodeId,
  });
}

// ============================================================================
// Failover Hooks
// ============================================================================

export function useFailoverHistory(limit: number = 50) {
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  const query = useQuery({
    queryKey: clusterKeys.failoverHistory(),
    queryFn: () => clusterService.getFailoverHistory(limit),
    staleTime: 30000,
  });

  // Subscribe to new failover events
  useEffect(() => {
    const unsubscribe = subscribe('cluster:failover:event', (data) => {
      queryClient.setQueryData<FailoverEvent[]>(
        clusterKeys.failoverHistory(),
        (old) => {
          if (!old) return [data as FailoverEvent];
          return [data as FailoverEvent, ...old].slice(0, limit);
        }
      );

      // Show notification
      const event = data as FailoverEvent;
      if (event.status === 'success') {
        toast.success(`Failover completed: New leader is ${event.newLeader}`);
      } else {
        toast.error(`Failover failed: ${event.details?.error || 'Unknown error'}`);
      }

      // Invalidate topology and nodes
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
    });

    return unsubscribe;
  }, [limit, subscribe, queryClient]);

  return query;
}

export function useFailoverConfig() {
  return useQuery({
    queryKey: clusterKeys.failoverConfig(),
    queryFn: clusterService.getFailoverConfig,
    staleTime: 60000,
  });
}

export function useUpdateFailoverConfig() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.updateFailoverConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.failoverConfig() });
      toast.success('Failover configuration updated');
    },
    onError: (error: Error) => {
      toast.error(`Failed to update failover config: ${error.message}`);
    },
  });
}

export function usePreflightFailoverCheck() {
  return useMutation({
    mutationFn: clusterService.preflightFailoverCheck,
    onError: (error: Error) => {
      toast.error(`Preflight check failed: ${error.message}`);
    },
  });
}

export function useTriggerFailover() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.triggerFailover,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.failoverHistory() });
      toast.success('Failover initiated');
    },
    onError: (error: Error) => {
      toast.error(`Failed to trigger failover: ${error.message}`);
    },
  });
}

export function useCancelFailover() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.cancelFailover,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.topology() });
      queryClient.invalidateQueries({ queryKey: clusterKeys.nodes() });
      toast.success('Failover cancelled');
    },
    onError: (error: Error) => {
      toast.error(`Failed to cancel failover: ${error.message}`);
    },
  });
}

// ============================================================================
// Health & Config Hooks
// ============================================================================

export function useClusterHealth() {
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  const query = useQuery({
    queryKey: clusterKeys.health(),
    queryFn: clusterService.getClusterHealth,
    refetchInterval: 5000,
    staleTime: 3000,
  });

  // Subscribe to health updates
  useEffect(() => {
    const unsubscribe = subscribe('cluster:health', (data) => {
      queryClient.setQueryData(clusterKeys.health(), data);
    });

    return unsubscribe;
  }, [subscribe, queryClient]);

  return query;
}

export function useClusterConfig() {
  return useQuery({
    queryKey: clusterKeys.config(),
    queryFn: clusterService.getClusterConfig,
    staleTime: 60000,
  });
}

export function useUpdateClusterConfig() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clusterService.updateClusterConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: clusterKeys.config() });
      toast.success('Cluster configuration updated');
    },
    onError: (error: Error) => {
      toast.error(`Failed to update cluster config: ${error.message}`);
    },
  });
}

export function useRunClusterDiagnostics() {
  return useMutation({
    mutationFn: clusterService.runClusterDiagnostics,
    onError: (error: Error) => {
      toast.error(`Failed to run diagnostics: ${error.message}`);
    },
  });
}
