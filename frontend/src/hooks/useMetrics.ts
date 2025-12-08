// ============================================================================
// RustyDB Metrics Hooks
// Custom React hooks for metrics with React Query integration
// ============================================================================

import { useQuery, useQueryClient, UseQueryResult } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import {
  metricsService,
  TimeRange,
  PerformanceHistory,
  ActivityEvent,
  ActivityEventType,
  MetricsSubscription,
} from '../services/metricsService';
import type {
  SystemMetrics,
  DatabaseMetrics,
  QueryMetrics,
  HealthStatus,
  Alert,
  ActiveSession,
  ConnectionPoolStats,
  SlowQuery,
} from '../types';

// ============================================================================
// Query Keys
// ============================================================================

export const metricsKeys = {
  all: ['metrics'] as const,
  system: () => [...metricsKeys.all, 'system'] as const,
  database: () => [...metricsKeys.all, 'database'] as const,
  queries: () => [...metricsKeys.all, 'queries'] as const,
  performance: (timeRange: TimeRange) =>
    [...metricsKeys.all, 'performance', timeRange] as const,
  health: () => [...metricsKeys.all, 'health'] as const,
  alerts: (acknowledged?: boolean) =>
    [...metricsKeys.all, 'alerts', acknowledged] as const,
  sessions: () => [...metricsKeys.all, 'sessions'] as const,
  connectionPools: () => [...metricsKeys.all, 'connection-pools'] as const,
  slowQueries: (limit: number) => [...metricsKeys.all, 'slow-queries', limit] as const,
  activity: (limit: number, offset: number, type?: ActivityEventType) =>
    [...metricsKeys.all, 'activity', limit, offset, type] as const,
};

// ============================================================================
// System Metrics Hook
// ============================================================================

export interface UseSystemMetricsOptions {
  enabled?: boolean;
  refetchInterval?: number;
  realtime?: boolean;
}

export function useSystemMetrics(
  options: UseSystemMetricsOptions = {}
): UseQueryResult<SystemMetrics> {
  const { enabled = true, refetchInterval = 5000, realtime = false } = options;
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<MetricsSubscription | null>(null);

  const query = useQuery({
    queryKey: metricsKeys.system(),
    queryFn: () => metricsService.fetchSystemMetrics(),
    enabled,
    refetchInterval: realtime ? false : refetchInterval,
    staleTime: 3000,
  });

  useEffect(() => {
    if (realtime && enabled) {
      subscriptionRef.current = metricsService.subscribeToMetrics((metrics) => {
        queryClient.setQueryData(metricsKeys.system(), metrics);
      });

      return () => {
        subscriptionRef.current?.unsubscribe();
      };
    }
  }, [realtime, enabled, queryClient]);

  return query;
}

// ============================================================================
// Database Metrics Hook
// ============================================================================

export interface UseDatabaseMetricsOptions {
  enabled?: boolean;
  refetchInterval?: number;
  realtime?: boolean;
}

export function useDatabaseMetrics(
  options: UseDatabaseMetricsOptions = {}
): UseQueryResult<DatabaseMetrics> {
  const { enabled = true, refetchInterval = 5000, realtime = false } = options;
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<MetricsSubscription | null>(null);

  const query = useQuery({
    queryKey: metricsKeys.database(),
    queryFn: () => metricsService.fetchDatabaseMetrics(),
    enabled,
    refetchInterval: realtime ? false : refetchInterval,
    staleTime: 3000,
  });

  useEffect(() => {
    if (realtime && enabled) {
      subscriptionRef.current = metricsService.subscribeToDatabaseMetrics((metrics) => {
        queryClient.setQueryData(metricsKeys.database(), metrics);
      });

      return () => {
        subscriptionRef.current?.unsubscribe();
      };
    }
  }, [realtime, enabled, queryClient]);

  return query;
}

// ============================================================================
// Query Metrics Hook
// ============================================================================

export interface UseQueryMetricsOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export function useQueryMetrics(
  options: UseQueryMetricsOptions = {}
): UseQueryResult<QueryMetrics> {
  const { enabled = true, refetchInterval = 10000 } = options;

  return useQuery({
    queryKey: metricsKeys.queries(),
    queryFn: () => metricsService.fetchQueryMetrics(),
    enabled,
    refetchInterval,
    staleTime: 5000,
  });
}

// ============================================================================
// Performance Chart Hook
// ============================================================================

export interface UsePerformanceChartOptions {
  timeRange: TimeRange;
  enabled?: boolean;
  refetchInterval?: number;
}

export function usePerformanceChart(
  options: UsePerformanceChartOptions
): UseQueryResult<PerformanceHistory> {
  const { timeRange, enabled = true, refetchInterval = 30000 } = options;

  return useQuery({
    queryKey: metricsKeys.performance(timeRange),
    queryFn: () => metricsService.fetchPerformanceHistory(timeRange),
    enabled,
    refetchInterval,
    staleTime: 10000,
  });
}

// ============================================================================
// Health Status Hook
// ============================================================================

export interface UseHealthStatusOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export function useHealthStatus(
  options: UseHealthStatusOptions = {}
): UseQueryResult<HealthStatus> {
  const { enabled = true, refetchInterval = 10000 } = options;

  return useQuery({
    queryKey: metricsKeys.health(),
    queryFn: () => metricsService.fetchHealthStatus(),
    enabled,
    refetchInterval,
    staleTime: 5000,
  });
}

// ============================================================================
// Alerts Hook
// ============================================================================

export interface UseAlertsOptions {
  acknowledged?: boolean;
  enabled?: boolean;
  refetchInterval?: number;
  realtime?: boolean;
}

export function useAlerts(options: UseAlertsOptions = {}): UseQueryResult<Alert[]> {
  const { acknowledged, enabled = true, refetchInterval = 10000, realtime = false } = options;
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<MetricsSubscription | null>(null);

  const query = useQuery({
    queryKey: metricsKeys.alerts(acknowledged),
    queryFn: () => metricsService.fetchAlerts(acknowledged),
    enabled,
    refetchInterval: realtime ? false : refetchInterval,
    staleTime: 5000,
  });

  useEffect(() => {
    if (realtime && enabled) {
      subscriptionRef.current = metricsService.subscribeToAlerts((alert) => {
        // Prepend new alert to the list
        queryClient.setQueryData<Alert[]>(metricsKeys.alerts(acknowledged), (old) => {
          return old ? [alert, ...old] : [alert];
        });
      });

      return () => {
        subscriptionRef.current?.unsubscribe();
      };
    }
  }, [realtime, enabled, acknowledged, queryClient]);

  return query;
}

// ============================================================================
// Active Sessions Hook
// ============================================================================

export interface UseActiveSessionsOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export function useActiveSessions(
  options: UseActiveSessionsOptions = {}
): UseQueryResult<ActiveSession[]> {
  const { enabled = true, refetchInterval = 10000 } = options;

  return useQuery({
    queryKey: metricsKeys.sessions(),
    queryFn: () => metricsService.fetchActiveSessions(),
    enabled,
    refetchInterval,
    staleTime: 5000,
  });
}

// ============================================================================
// Connection Pool Stats Hook
// ============================================================================

export interface UseConnectionPoolStatsOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export function useConnectionPoolStats(
  options: UseConnectionPoolStatsOptions = {}
): UseQueryResult<ConnectionPoolStats[]> {
  const { enabled = true, refetchInterval = 10000 } = options;

  return useQuery({
    queryKey: metricsKeys.connectionPools(),
    queryFn: () => metricsService.fetchConnectionPoolStats(),
    enabled,
    refetchInterval,
    staleTime: 5000,
  });
}

// ============================================================================
// Slow Queries Hook
// ============================================================================

export interface UseSlowQueriesOptions {
  limit?: number;
  enabled?: boolean;
  refetchInterval?: number;
}

export function useSlowQueries(
  options: UseSlowQueriesOptions = {}
): UseQueryResult<SlowQuery[]> {
  const { limit = 20, enabled = true, refetchInterval = 30000 } = options;

  return useQuery({
    queryKey: metricsKeys.slowQueries(limit),
    queryFn: () => metricsService.fetchSlowQueries(limit),
    enabled,
    refetchInterval,
    staleTime: 10000,
  });
}

// ============================================================================
// Activity Events Hook
// ============================================================================

export interface UseActivityEventsOptions {
  limit?: number;
  offset?: number;
  type?: ActivityEventType;
  enabled?: boolean;
  refetchInterval?: number;
  realtime?: boolean;
}

export function useActivityEvents(
  options: UseActivityEventsOptions = {}
): UseQueryResult<{ events: ActivityEvent[]; total: number }> & {
  hasMore: boolean;
} {
  const {
    limit = 50,
    offset = 0,
    type,
    enabled = true,
    refetchInterval = 30000,
    realtime = false,
  } = options;
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<MetricsSubscription | null>(null);

  const query = useQuery({
    queryKey: metricsKeys.activity(limit, offset, type),
    queryFn: () => metricsService.fetchActivityEvents(limit, offset, type),
    enabled,
    refetchInterval: realtime ? false : refetchInterval,
    staleTime: 10000,
  });

  useEffect(() => {
    if (realtime && enabled && offset === 0) {
      subscriptionRef.current = metricsService.subscribeToActivity((event) => {
        // Prepend new event to the list
        queryClient.setQueryData<{ events: ActivityEvent[]; total: number }>(
          metricsKeys.activity(limit, offset, type),
          (old) => {
            if (!old) return { events: [event], total: 1 };
            return {
              events: [event, ...old.events].slice(0, limit),
              total: old.total + 1,
            };
          }
        );
      });

      return () => {
        subscriptionRef.current?.unsubscribe();
      };
    }
  }, [realtime, enabled, offset, limit, type, queryClient]);

  const hasMore = query.data ? offset + limit < query.data.total : false;

  return { ...query, hasMore };
}

// ============================================================================
// Composite Dashboard Metrics Hook
// ============================================================================

export interface UseDashboardMetricsOptions {
  enabled?: boolean;
  realtime?: boolean;
}

export interface DashboardMetrics {
  system: SystemMetrics | undefined;
  database: DatabaseMetrics | undefined;
  queries: QueryMetrics | undefined;
  health: HealthStatus | undefined;
  alerts: Alert[] | undefined;
  isLoading: boolean;
  isError: boolean;
  error: Error | null;
}

export function useDashboardMetrics(
  options: UseDashboardMetricsOptions = {}
): DashboardMetrics {
  const { enabled = true, realtime = false } = options;

  const systemQuery = useSystemMetrics({ enabled, realtime });
  const databaseQuery = useDatabaseMetrics({ enabled, realtime });
  const queriesQuery = useQueryMetrics({ enabled });
  const healthQuery = useHealthStatus({ enabled });
  const alertsQuery = useAlerts({ enabled, acknowledged: false, realtime });

  const isLoading =
    systemQuery.isLoading ||
    databaseQuery.isLoading ||
    queriesQuery.isLoading ||
    healthQuery.isLoading ||
    alertsQuery.isLoading;

  const isError =
    systemQuery.isError ||
    databaseQuery.isError ||
    queriesQuery.isError ||
    healthQuery.isError ||
    alertsQuery.isError;

  const error =
    (systemQuery.error ||
      databaseQuery.error ||
      queriesQuery.error ||
      healthQuery.error ||
      alertsQuery.error) as Error | null;

  return {
    system: systemQuery.data,
    database: databaseQuery.data,
    queries: queriesQuery.data,
    health: healthQuery.data,
    alerts: alertsQuery.data,
    isLoading,
    isError,
    error,
  };
}

// ============================================================================
// Metric Change Calculator Hook
// ============================================================================

export interface MetricChange {
  current: number;
  previous: number;
  change: number;
  changePercent: number;
  trend: 'up' | 'down' | 'stable';
}

export function useMetricChange(
  current: number | undefined,
  previous: number | undefined
): MetricChange | null {
  if (current === undefined || previous === undefined) {
    return null;
  }

  const change = current - previous;
  const changePercent = previous === 0 ? 0 : (change / previous) * 100;

  let trend: 'up' | 'down' | 'stable';
  if (Math.abs(changePercent) < 1) {
    trend = 'stable';
  } else if (change > 0) {
    trend = 'up';
  } else {
    trend = 'down';
  }

  return {
    current,
    previous,
    change,
    changePercent,
    trend,
  };
}

// ============================================================================
// Cleanup Effect
// ============================================================================

export function useMetricsCleanup(): void {
  useEffect(() => {
    return () => {
      // Cleanup WebSocket connections when component unmounts
      metricsService.disconnectWebSocket();
    };
  }, []);
}
