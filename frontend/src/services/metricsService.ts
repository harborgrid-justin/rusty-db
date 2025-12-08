// ============================================================================
// RustyDB Metrics Service
// API service for system and database metrics with WebSocket support
// ============================================================================

import { get, WS_URL } from './api';
import type {
  SystemMetrics,
  DatabaseMetrics,
  QueryMetrics,
  HealthStatus,
  Alert,
  ActiveSession,
  ConnectionPoolStats,
  SlowQuery,
  ApiResponse,
  Duration,
  Timestamp,
} from '../types';

// ============================================================================
// Types for Metrics Service
// ============================================================================

export interface PerformanceDataPoint {
  timestamp: Timestamp;
  value: number;
  label?: string;
}

export interface PerformanceHistory {
  timeRange: TimeRange;
  metrics: {
    queriesPerSecond: PerformanceDataPoint[];
    transactionsPerSecond: PerformanceDataPoint[];
    avgResponseTime: PerformanceDataPoint[];
    activeConnections: PerformanceDataPoint[];
    cpuUsage: PerformanceDataPoint[];
    memoryUsage: PerformanceDataPoint[];
    diskUsage: PerformanceDataPoint[];
    cacheHitRatio: PerformanceDataPoint[];
  };
}

export type TimeRange = '1h' | '6h' | '24h' | '7d' | '30d';

export interface MetricsSubscription {
  id: string;
  unsubscribe: () => void;
}

export interface ActivityEvent {
  id: string;
  type: ActivityEventType;
  title: string;
  description: string;
  timestamp: Timestamp;
  severity: 'info' | 'warning' | 'error' | 'success';
  metadata?: Record<string, unknown>;
  userId?: string;
  username?: string;
}

export type ActivityEventType =
  | 'query_executed'
  | 'backup_created'
  | 'backup_failed'
  | 'user_login'
  | 'user_logout'
  | 'configuration_changed'
  | 'alert_triggered'
  | 'alert_resolved'
  | 'connection_limit_reached'
  | 'slow_query_detected'
  | 'replication_lag'
  | 'failover_completed'
  | 'index_created'
  | 'table_created'
  | 'maintenance_started'
  | 'maintenance_completed';

// ============================================================================
// WebSocket Connection Management
// ============================================================================

class MetricsWebSocket {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private subscribers = new Map<string, Set<(data: unknown) => void>>();
  private isConnecting = false;

  constructor() {
    this.connect();
  }

  private connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN || this.isConnecting) {
      return;
    }

    this.isConnecting = true;

    try {
      this.ws = new WebSocket(`${WS_URL}/metrics`);

      this.ws.onopen = () => {
        console.log('Metrics WebSocket connected');
        this.isConnecting = false;
        this.reconnectAttempts = 0;
        this.reconnectDelay = 1000;
      };

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          const { channel, payload } = data;

          if (channel && this.subscribers.has(channel)) {
            this.subscribers.get(channel)?.forEach((callback) => {
              callback(payload);
            });
          }
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };

      this.ws.onerror = (error) => {
        console.error('Metrics WebSocket error:', error);
        this.isConnecting = false;
      };

      this.ws.onclose = () => {
        console.log('Metrics WebSocket disconnected');
        this.isConnecting = false;
        this.ws = null;
        this.attemptReconnect();
      };
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
      this.isConnecting = false;
      this.attemptReconnect();
    }
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      console.log(
        `Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`
      );

      setTimeout(() => {
        this.connect();
      }, this.reconnectDelay);

      // Exponential backoff
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000);
    } else {
      console.error('Max reconnection attempts reached');
    }
  }

  subscribe(channel: string, callback: (data: unknown) => void): () => void {
    if (!this.subscribers.has(channel)) {
      this.subscribers.set(channel, new Set());
    }

    this.subscribers.get(channel)!.add(callback);

    // Send subscription message
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ action: 'subscribe', channel }));
    }

    // Return unsubscribe function
    return () => {
      const channelSubs = this.subscribers.get(channel);
      if (channelSubs) {
        channelSubs.delete(callback);
        if (channelSubs.size === 0) {
          this.subscribers.delete(channel);
          // Send unsubscribe message
          if (this.ws?.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ action: 'unsubscribe', channel }));
          }
        }
      }
    };
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.subscribers.clear();
  }
}

// Global WebSocket instance
let metricsWS: MetricsWebSocket | null = null;

function getMetricsWebSocket(): MetricsWebSocket {
  if (!metricsWS) {
    metricsWS = new MetricsWebSocket();
  }
  return metricsWS;
}

// ============================================================================
// Metrics Service API
// ============================================================================

export const metricsService = {
  /**
   * Fetch current system metrics (CPU, Memory, Disk, Network)
   */
  async fetchSystemMetrics(): Promise<SystemMetrics> {
    const response = await get<SystemMetrics>('/metrics/system');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch system metrics');
    }

    return response.data;
  },

  /**
   * Fetch database-specific metrics
   */
  async fetchDatabaseMetrics(): Promise<DatabaseMetrics> {
    const response = await get<DatabaseMetrics>('/metrics/database');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch database metrics');
    }

    return response.data;
  },

  /**
   * Fetch query statistics
   */
  async fetchQueryMetrics(): Promise<QueryMetrics> {
    const response = await get<QueryMetrics>('/metrics/queries');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch query metrics');
    }

    return response.data;
  },

  /**
   * Fetch performance history for a given time range
   */
  async fetchPerformanceHistory(timeRange: TimeRange): Promise<PerformanceHistory> {
    const response = await get<PerformanceHistory>(
      `/metrics/performance?timeRange=${timeRange}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch performance history');
    }

    return response.data;
  },

  /**
   * Fetch system health status
   */
  async fetchHealthStatus(): Promise<HealthStatus> {
    const response = await get<HealthStatus>('/health');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch health status');
    }

    return response.data;
  },

  /**
   * Fetch active alerts
   */
  async fetchAlerts(acknowledged?: boolean): Promise<Alert[]> {
    const params = acknowledged !== undefined ? `?acknowledged=${acknowledged}` : '';
    const response = await get<Alert[]>(`/metrics/alerts${params}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch alerts');
    }

    return response.data;
  },

  /**
   * Fetch active sessions
   */
  async fetchActiveSessions(): Promise<ActiveSession[]> {
    const response = await get<ActiveSession[]>('/metrics/sessions');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch active sessions');
    }

    return response.data;
  },

  /**
   * Fetch connection pool statistics
   */
  async fetchConnectionPoolStats(): Promise<ConnectionPoolStats[]> {
    const response = await get<ConnectionPoolStats[]>('/metrics/connection-pools');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch connection pool stats');
    }

    return response.data;
  },

  /**
   * Fetch slow queries
   */
  async fetchSlowQueries(limit = 20): Promise<SlowQuery[]> {
    const response = await get<SlowQuery[]>(`/metrics/slow-queries?limit=${limit}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch slow queries');
    }

    return response.data;
  },

  /**
   * Fetch recent activity events
   */
  async fetchActivityEvents(
    limit = 50,
    offset = 0,
    type?: ActivityEventType
  ): Promise<{ events: ActivityEvent[]; total: number }> {
    const typeParam = type ? `&type=${type}` : '';
    const response = await get<{ events: ActivityEvent[]; total: number }>(
      `/metrics/activity?limit=${limit}&offset=${offset}${typeParam}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch activity events');
    }

    return response.data;
  },

  /**
   * Subscribe to real-time metrics updates
   */
  subscribeToMetrics(callback: (metrics: SystemMetrics) => void): MetricsSubscription {
    const ws = getMetricsWebSocket();
    const id = crypto.randomUUID();
    const unsubscribe = ws.subscribe('system_metrics', (data) => {
      callback(data as SystemMetrics);
    });

    return { id, unsubscribe };
  },

  /**
   * Subscribe to database metrics updates
   */
  subscribeToDatabaseMetrics(
    callback: (metrics: DatabaseMetrics) => void
  ): MetricsSubscription {
    const ws = getMetricsWebSocket();
    const id = crypto.randomUUID();
    const unsubscribe = ws.subscribe('database_metrics', (data) => {
      callback(data as DatabaseMetrics);
    });

    return { id, unsubscribe };
  },

  /**
   * Subscribe to alerts
   */
  subscribeToAlerts(callback: (alert: Alert) => void): MetricsSubscription {
    const ws = getMetricsWebSocket();
    const id = crypto.randomUUID();
    const unsubscribe = ws.subscribe('alerts', (data) => {
      callback(data as Alert);
    });

    return { id, unsubscribe };
  },

  /**
   * Subscribe to activity events
   */
  subscribeToActivity(callback: (event: ActivityEvent) => void): MetricsSubscription {
    const ws = getMetricsWebSocket();
    const id = crypto.randomUUID();
    const unsubscribe = ws.subscribe('activity', (data) => {
      callback(data as ActivityEvent);
    });

    return { id, unsubscribe };
  },

  /**
   * Disconnect all WebSocket subscriptions
   */
  disconnectWebSocket(): void {
    if (metricsWS) {
      metricsWS.disconnect();
      metricsWS = null;
    }
  },
};

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Format bytes to human-readable string
 */
export function formatBytes(bytes: number, decimals = 2): string {
  if (bytes === 0) return '0 Bytes';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

/**
 * Format duration to human-readable string
 */
export function formatDuration(duration: Duration): string {
  if (duration < 1000) {
    return `${duration}ms`;
  }

  const seconds = Math.floor(duration / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) {
    return `${days}d ${hours % 24}h`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  }
  return `${seconds}s`;
}

/**
 * Format percentage
 */
export function formatPercentage(value: number, decimals = 1): string {
  return `${value.toFixed(decimals)}%`;
}

/**
 * Get health status color
 */
export function getHealthStatusColor(status: HealthStatus['status']): string {
  const colors = {
    healthy: 'text-green-600',
    degraded: 'text-yellow-600',
    unhealthy: 'text-orange-600',
    critical: 'text-red-600',
  };
  return colors[status] || 'text-gray-600';
}

/**
 * Get alert severity color
 */
export function getAlertSeverityColor(severity: Alert['severity']): string {
  const colors = {
    info: 'text-blue-600',
    warning: 'text-yellow-600',
    error: 'text-orange-600',
    critical: 'text-red-600',
  };
  return colors[severity] || 'text-gray-600';
}
