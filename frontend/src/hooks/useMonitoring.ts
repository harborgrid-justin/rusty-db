import { useState, useEffect, useCallback, useRef } from 'react';
import {
  monitoringService,
  type PerformanceMetricsResponse,
  type SessionFilters,
  type SlowQueryFilters,
  type AlertFilters,
  type BlockingTree,
  type LockStats,
  type MonitoringWebSocket,
} from '../services/monitoringService';
import type {
  SystemMetrics,
  ActiveSession,
  SlowQuery,
  Alert,
  PaginatedResponse,
  UUID,
} from '../types';

// ============================================================================
// Performance Metrics Hook
// ============================================================================

export function usePerformanceMetrics(
  intervalMinutes: number = 60,
  refreshInterval: number = 30000
) {
  const [data, setData] = useState<PerformanceMetricsResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchMetrics = useCallback(async () => {
    try {
      setError(null);
      const metrics = await monitoringService.getPerformanceMetrics(intervalMinutes);
      setData(metrics);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch metrics');
    } finally {
      setIsLoading(false);
    }
  }, [intervalMinutes]);

  useEffect(() => {
    fetchMetrics();

    const interval = setInterval(fetchMetrics, refreshInterval);

    return () => clearInterval(interval);
  }, [fetchMetrics, refreshInterval]);

  return {
    data,
    isLoading,
    error,
    refresh: fetchMetrics,
  };
}

// ============================================================================
// Current Metrics Hook with WebSocket
// ============================================================================

export function useCurrentMetrics(enableWebSocket: boolean = true) {
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<MonitoringWebSocket | null>(null);

  const fetchCurrentMetrics = useCallback(async () => {
    try {
      setError(null);
      const data = await monitoringService.getCurrentMetrics();
      setMetrics(data);
      setIsLoading(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch current metrics');
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchCurrentMetrics();

    if (enableWebSocket) {
      const ws = monitoringService.createWebSocket();
      wsRef.current = ws;

      ws.connect((data: unknown) => {
        const message = data as { type: string; payload: SystemMetrics };
        if (message.type === 'metrics_update') {
          setMetrics(message.payload);
        }
      });

      ws.subscribe('metrics');

      return () => {
        ws.disconnect();
      };
    } else {
      // Fallback to polling if WebSocket is disabled
      const interval = setInterval(fetchCurrentMetrics, 5000);
      return () => clearInterval(interval);
    }
  }, [fetchCurrentMetrics, enableWebSocket]);

  return {
    metrics,
    isLoading,
    error,
    refresh: fetchCurrentMetrics,
  };
}

// ============================================================================
// Active Sessions Hook
// ============================================================================

export function useActiveSessions(
  filters?: SessionFilters,
  refreshInterval: number = 10000
) {
  const [sessions, setSessions] = useState<ActiveSession[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchSessions = useCallback(async () => {
    try {
      setError(null);
      const data = await monitoringService.getActiveSessions(filters);
      setSessions(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch sessions');
    } finally {
      setIsLoading(false);
    }
  }, [filters]);

  useEffect(() => {
    fetchSessions();

    const interval = setInterval(fetchSessions, refreshInterval);

    return () => clearInterval(interval);
  }, [fetchSessions, refreshInterval]);

  const killSession = useCallback(
    async (sessionId: UUID, force: boolean = false) => {
      try {
        await monitoringService.killSession(sessionId, force);
        await fetchSessions(); // Refresh the list
      } catch (err) {
        throw new Error(
          err instanceof Error ? err.message : 'Failed to kill session'
        );
      }
    },
    [fetchSessions]
  );

  return {
    sessions,
    isLoading,
    error,
    refresh: fetchSessions,
    killSession,
  };
}

// ============================================================================
// Slow Queries Hook
// ============================================================================

export function useSlowQueries(filters?: SlowQueryFilters) {
  const [data, setData] = useState<PaginatedResponse<SlowQuery> | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchSlowQueries = useCallback(async () => {
    try {
      setError(null);
      setIsLoading(true);
      const result = await monitoringService.getSlowQueries(filters);
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch slow queries');
    } finally {
      setIsLoading(false);
    }
  }, [filters]);

  useEffect(() => {
    fetchSlowQueries();
  }, [fetchSlowQueries]);

  const exportQueries = useCallback(
    async (format: 'json' | 'csv' = 'csv') => {
      try {
        const blob = await monitoringService.exportSlowQueries(filters, format);
        const url = window.URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `slow-queries-${new Date().toISOString()}.${format}`;
        document.body.appendChild(a);
        a.click();
        window.URL.revokeObjectURL(url);
        document.body.removeChild(a);
      } catch (err) {
        throw new Error(
          err instanceof Error ? err.message : 'Failed to export slow queries'
        );
      }
    },
    [filters]
  );

  return {
    data,
    isLoading,
    error,
    refresh: fetchSlowQueries,
    exportQueries,
  };
}

// ============================================================================
// Alerts Hook
// ============================================================================

export function useAlerts(
  filters?: AlertFilters,
  enableWebSocket: boolean = true
) {
  const [data, setData] = useState<PaginatedResponse<Alert> | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<MonitoringWebSocket | null>(null);

  const fetchAlerts = useCallback(async () => {
    try {
      setError(null);
      const result = await monitoringService.getAlerts(filters);
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch alerts');
    } finally {
      setIsLoading(false);
    }
  }, [filters]);

  useEffect(() => {
    fetchAlerts();

    if (enableWebSocket) {
      const ws = monitoringService.createWebSocket();
      wsRef.current = ws;

      ws.connect((message: unknown) => {
        const data = message as { type: string; payload: Alert };
        if (data.type === 'new_alert') {
          // Refresh alerts when new alert arrives
          fetchAlerts();
        }
      });

      ws.subscribe('alerts');

      return () => {
        ws.disconnect();
      };
    }
  }, [fetchAlerts, enableWebSocket]);

  const acknowledgeAlert = useCallback(
    async (alertId: UUID, note?: string) => {
      try {
        await monitoringService.acknowledgeAlert(alertId, note);
        await fetchAlerts(); // Refresh the list
      } catch (err) {
        throw new Error(
          err instanceof Error ? err.message : 'Failed to acknowledge alert'
        );
      }
    },
    [fetchAlerts]
  );

  const resolveAlert = useCallback(
    async (alertId: UUID, resolution?: string) => {
      try {
        await monitoringService.resolveAlert(alertId, resolution);
        await fetchAlerts(); // Refresh the list
      } catch (err) {
        throw new Error(
          err instanceof Error ? err.message : 'Failed to resolve alert'
        );
      }
    },
    [fetchAlerts]
  );

  return {
    data,
    isLoading,
    error,
    refresh: fetchAlerts,
    acknowledgeAlert,
    resolveAlert,
  };
}

// ============================================================================
// Blocking Tree Hook
// ============================================================================

export function useBlockingTree(refreshInterval: number = 15000) {
  const [data, setData] = useState<BlockingTree | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchBlockingTree = useCallback(async () => {
    try {
      setError(null);
      const tree = await monitoringService.getBlockingTree();
      setData(tree);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch blocking tree');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchBlockingTree();

    const interval = setInterval(fetchBlockingTree, refreshInterval);

    return () => clearInterval(interval);
  }, [fetchBlockingTree, refreshInterval]);

  return {
    data,
    isLoading,
    error,
    refresh: fetchBlockingTree,
  };
}

// ============================================================================
// Lock Statistics Hook
// ============================================================================

export function useLockStats(refreshInterval: number = 10000) {
  const [data, setData] = useState<LockStats | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchLockStats = useCallback(async () => {
    try {
      setError(null);
      const stats = await monitoringService.getLockStats();
      setData(stats);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch lock stats');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchLockStats();

    const interval = setInterval(fetchLockStats, refreshInterval);

    return () => clearInterval(interval);
  }, [fetchLockStats, refreshInterval]);

  return {
    data,
    isLoading,
    error,
    refresh: fetchLockStats,
  };
}
