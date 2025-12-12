import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  getResourceGroups,
  createResourceGroup,
  updateResourceGroup,
  deleteResourceGroup,
  getAllResourceUsage,
  getConnectionPools,
  createPool,
  updatePool,
  deletePool,
  drainPool as drainPoolApi,
  CreateResourceGroupRequest,
  UpdateResourceGroupRequest,
  CreatePoolRequest,
  UpdatePoolRequest,
} from '../services/resourceService';
import { getErrorMessage } from '../services/api';
import type { ResourceGroup, ResourceUsage, ConnectionPoolStats } from '@/types';

// ============================================================================
// Resource Management Hook
// Manages resource groups and connection pools state
// ============================================================================

interface UseResourcesReturn {
  // Resource Groups
  groups: ResourceGroup[];
  usageMap: Map<string, ResourceUsage>;
  selectedGroup: ResourceGroup | null;

  // Connection Pools
  pools: ConnectionPoolStats[];
  selectedPool: ConnectionPoolStats | null;

  // Loading & Error States
  loading: boolean;
  error: string | null;

  // Resource Group Actions
  createGroup: (config: CreateResourceGroupRequest) => Promise<void>;
  updateGroup: (id: string, config: UpdateResourceGroupRequest) => Promise<void>;
  deleteGroup: (id: string) => Promise<void>;
  selectGroup: (group: ResourceGroup | null) => void;
  refreshGroups: () => Promise<void>;

  // Connection Pool Actions
  createPool: (config: CreatePoolRequest) => Promise<void>;
  updatePool: (id: string, config: UpdatePoolRequest) => Promise<void>;
  deletePool: (id: string) => Promise<void>;
  drainPool: (id: string) => Promise<void>;
  selectPool: (pool: ConnectionPoolStats | null) => void;
  refreshPools: () => Promise<void>;

  // Combined Actions
  refreshAll: () => Promise<void>;
}

export function useResources(): UseResourcesReturn {
  const navigate = useNavigate();

  // Resource Groups State
  const [groups, setGroups] = useState<ResourceGroup[]>([]);
  const [usageData, setUsageData] = useState<ResourceUsage[]>([]);
  const [selectedGroup, setSelectedGroup] = useState<ResourceGroup | null>(null);

  // Connection Pools State
  const [pools, setPools] = useState<ConnectionPoolStats[]>([]);
  const [selectedPool, setSelectedPool] = useState<ConnectionPoolStats | null>(null);

  // Common State
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Convert usage array to map for easier lookup
  const usageMap = new Map(usageData.map((usage) => [usage.groupId, usage]));

  // ============================================================================
  // Resource Groups Operations
  // ============================================================================

  const fetchGroups = useCallback(async () => {
    try {
      setError(null);
      const [groupsData, usageData] = await Promise.all([
        getResourceGroups(),
        getAllResourceUsage(),
      ]);
      setGroups(groupsData);
      setUsageData(usageData);
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      console.error('Failed to fetch resource groups:', err);
    }
  }, []);

  const createGroupHandler = useCallback(
    async (config: CreateResourceGroupRequest) => {
      try {
        setError(null);
        const newGroup = await createResourceGroup(config);
        setGroups((prev) => [...prev, newGroup]);
      } catch (err) {
        const message = getErrorMessage(err);
        setError(message);
        throw err;
      }
    },
    []
  );

  const updateGroupHandler = useCallback(
    async (id: string, config: UpdateResourceGroupRequest) => {
      try {
        setError(null);
        const updatedGroup = await updateResourceGroup(id, config);
        setGroups((prev) =>
          prev.map((group) => (group.id === id ? updatedGroup : group))
        );
      } catch (err) {
        const message = getErrorMessage(err);
        setError(message);
        throw err;
      }
    },
    []
  );

  const deleteGroupHandler = useCallback(async (id: string) => {
    try {
      setError(null);
      await deleteResourceGroup(id);
      setGroups((prev) => prev.filter((group) => group.id !== id));
      if (selectedGroup?.id === id) {
        setSelectedGroup(null);
      }
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      throw err;
    }
  }, [selectedGroup]);

  const refreshGroupsHandler = useCallback(async () => {
    setLoading(true);
    await fetchGroups();
    setLoading(false);
  }, [fetchGroups]);

  // ============================================================================
  // Connection Pools Operations
  // ============================================================================

  const fetchPools = useCallback(async () => {
    try {
      setError(null);
      const poolsData = await getConnectionPools();
      setPools(poolsData);
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      console.error('Failed to fetch connection pools:', err);
    }
  }, []);

  const createPoolHandler = useCallback(async (config: CreatePoolRequest) => {
    try {
      setError(null);
      const newPool = await createPool(config);
      setPools((prev) => [...prev, newPool]);
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      throw err;
    }
  }, []);

  const updatePoolHandler = useCallback(
    async (id: string, config: UpdatePoolRequest) => {
      try {
        setError(null);
        const updatedPool = await updatePool(id, config);
        setPools((prev) =>
          prev.map((pool) => (pool.poolId === id ? updatedPool : pool))
        );
      } catch (err) {
        const message = getErrorMessage(err);
        setError(message);
        throw err;
      }
    },
    []
  );

  const deletePoolHandler = useCallback(
    async (id: string) => {
      try {
        setError(null);
        await deletePool(id);
        setPools((prev) => prev.filter((pool) => pool.poolId !== id));
        if (selectedPool?.poolId === id) {
          setSelectedPool(null);
        }
      } catch (err) {
        const message = getErrorMessage(err);
        setError(message);
        throw err;
      }
    },
    [selectedPool]
  );

  const drainPoolHandler = useCallback(async (id: string) => {
    try {
      setError(null);
      await drainPoolApi(id);
      // Refresh pools to get updated status
      await fetchPools();
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      throw err;
    }
  }, [fetchPools]);

  const refreshPoolsHandler = useCallback(async () => {
    setLoading(true);
    await fetchPools();
    setLoading(false);
  }, [fetchPools]);

  // ============================================================================
  // Combined Operations
  // ============================================================================

  const refreshAll = useCallback(async () => {
    setLoading(true);
    await Promise.all([fetchGroups(), fetchPools()]);
    setLoading(false);
  }, [fetchGroups, fetchPools]);

  // ============================================================================
  // Effects
  // ============================================================================

  // Initial data fetch
  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      await Promise.all([fetchGroups(), fetchPools()]);
      setLoading(false);
    };

    loadData();
  }, [fetchGroups, fetchPools]);

  // Auto-refresh usage data every 30 seconds
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const usageData = await getAllResourceUsage();
        setUsageData(usageData);
      } catch (err) {
        console.error('Failed to refresh usage data:', err);
      }
    }, 30000); // 30 seconds

    return () => clearInterval(interval);
  }, []);

  // Auto-refresh pool stats every 15 seconds
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const poolsData = await getConnectionPools();
        setPools(poolsData);
      } catch (err) {
        console.error('Failed to refresh pool stats:', err);
      }
    }, 15000); // 15 seconds

    return () => clearInterval(interval);
  }, []);

  return {
    // Resource Groups
    groups,
    usageMap,
    selectedGroup,

    // Connection Pools
    pools,
    selectedPool,

    // Loading & Error
    loading,
    error,

    // Resource Group Actions
    createGroup: createGroupHandler,
    updateGroup: updateGroupHandler,
    deleteGroup: deleteGroupHandler,
    selectGroup: setSelectedGroup,
    refreshGroups: refreshGroupsHandler,

    // Connection Pool Actions
    createPool: createPoolHandler,
    updatePool: updatePoolHandler,
    deletePool: deletePoolHandler,
    drainPool: drainPoolHandler,
    selectPool: setSelectedPool,
    refreshPools: refreshPoolsHandler,

    // Combined
    refreshAll,
  };
}

// ============================================================================
// Specialized Hooks
// ============================================================================

/**
 * Hook for managing a single resource group
 */
export function useResourceGroup(groupId: string) {
  const { groups, usageMap, updateGroup, deleteGroup } = useResources();

  const group = groups.find((g) => g.id === groupId);
  const usage = group ? usageMap.get(group.id) : undefined;

  return {
    group,
    usage,
    updateGroup: (config: UpdateResourceGroupRequest) => updateGroup(groupId, config),
    deleteGroup: () => deleteGroup(groupId),
  };
}

/**
 * Hook for managing a single connection pool
 */
export function useConnectionPool(poolId: string) {
  const { pools, updatePool, deletePool, drainPool } = useResources();

  const pool = pools.find((p) => p.poolId === poolId);

  return {
    pool,
    updatePool: (config: UpdatePoolRequest) => updatePool(poolId, config),
    deletePool: () => deletePool(poolId),
    drainPool: () => drainPool(poolId),
  };
}
