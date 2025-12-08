import { get, post, del, buildQueryParams, WS_URL } from './api';
import type {
  SystemMetrics,
  ActiveSession,
  SlowQuery,
  Alert,
  UUID,
  PaginatedResponse,
  PaginationParams,
} from '../types';

// ============================================================================
// Monitoring Service API
// ============================================================================

export interface PerformanceMetricsResponse {
  current: SystemMetrics;
  history: SystemMetrics[];
  intervalSeconds: number;
}

export interface SessionFilters {
  state?: string;
  user?: string;
  database?: string;
  search?: string;
}

export interface SlowQueryFilters extends PaginationParams {
  database?: string;
  user?: string;
  minDuration?: number;
  startDate?: string;
  endDate?: string;
  search?: string;
}

export interface AlertFilters extends PaginationParams {
  severity?: string;
  type?: string;
  acknowledged?: boolean;
  resolved?: boolean;
  startDate?: string;
  endDate?: string;
}

export interface BlockingInfo {
  blockingSession: UUID;
  blockedSession: UUID;
  lockType: string;
  waitTime: number;
  query?: string;
}

export interface BlockingTree {
  nodes: BlockingTreeNode[];
  timestamp: string;
}

export interface BlockingTreeNode {
  sessionId: UUID;
  blockedBy?: UUID;
  blocking: UUID[];
  lockType: string;
  waitTime: number;
  query?: string;
  state: string;
}

export interface LockStats {
  totalLocks: number;
  grantedLocks: number;
  waitingLocks: number;
  locksByType: Record<string, number>;
  deadlocks: number;
  avgWaitTime: number;
  timestamp: string;
}

export interface AlertRule {
  id: UUID;
  name: string;
  type: string;
  condition: string;
  threshold: number;
  severity: string;
  enabled: boolean;
  notificationChannels: string[];
}

export interface MonitoringWebSocket {
  connect: (onMessage: (data: unknown) => void) => void;
  disconnect: () => void;
  subscribe: (topic: string) => void;
  unsubscribe: (topic: string) => void;
}

export const monitoringService = {
  /**
   * Get current and historical performance metrics
   */
  async getPerformanceMetrics(
    intervalMinutes: number = 60
  ): Promise<PerformanceMetricsResponse> {
    const response = await get<PerformanceMetricsResponse>(
      `/monitoring/metrics${buildQueryParams({ interval: intervalMinutes })}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch performance metrics');
    }

    return response.data;
  },

  /**
   * Get real-time system metrics
   */
  async getCurrentMetrics(): Promise<SystemMetrics> {
    const response = await get<SystemMetrics>('/monitoring/metrics/current');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch current metrics');
    }

    return response.data;
  },

  /**
   * Get all active database sessions
   */
  async getActiveSessions(filters?: SessionFilters): Promise<ActiveSession[]> {
    const queryString = filters ? buildQueryParams(filters) : '';
    const response = await get<ActiveSession[]>(`/monitoring/sessions${queryString}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch active sessions');
    }

    return response.data;
  },

  /**
   * Get details of a specific session
   */
  async getSessionDetails(sessionId: UUID): Promise<ActiveSession> {
    const response = await get<ActiveSession>(`/monitoring/sessions/${sessionId}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch session details');
    }

    return response.data;
  },

  /**
   * Terminate a database session
   */
  async killSession(sessionId: UUID, force: boolean = false): Promise<void> {
    const response = await del<void>(
      `/monitoring/sessions/${sessionId}${buildQueryParams({ force })}`
    );

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to kill session');
    }
  },

  /**
   * Get slow queries with filtering and pagination
   */
  async getSlowQueries(
    filters?: SlowQueryFilters
  ): Promise<PaginatedResponse<SlowQuery>> {
    const queryString = filters ? buildQueryParams(filters) : '';
    const response = await get<PaginatedResponse<SlowQuery>>(
      `/monitoring/slow-queries${queryString}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch slow queries');
    }

    return response.data;
  },

  /**
   * Get explain plan for a slow query
   */
  async getSlowQueryExplain(queryId: UUID): Promise<SlowQuery> {
    const response = await get<SlowQuery>(`/monitoring/slow-queries/${queryId}/explain`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch query explain plan');
    }

    return response.data;
  },

  /**
   * Export slow query report
   */
  async exportSlowQueries(
    filters?: SlowQueryFilters,
    format: 'json' | 'csv' = 'csv'
  ): Promise<Blob> {
    const queryString = filters ? buildQueryParams({ ...filters, format }) : '';
    const response = await get<Blob>(`/monitoring/slow-queries/export${queryString}`);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to export slow queries');
    }

    return response.data;
  },

  /**
   * Get alerts with filtering and pagination
   */
  async getAlerts(filters?: AlertFilters): Promise<PaginatedResponse<Alert>> {
    const queryString = filters ? buildQueryParams(filters) : '';
    const response = await get<PaginatedResponse<Alert>>(
      `/monitoring/alerts${queryString}`
    );

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch alerts');
    }

    return response.data;
  },

  /**
   * Acknowledge an alert
   */
  async acknowledgeAlert(alertId: UUID, note?: string): Promise<Alert> {
    const response = await post<Alert>(`/monitoring/alerts/${alertId}/acknowledge`, {
      note,
    });

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to acknowledge alert');
    }

    return response.data;
  },

  /**
   * Resolve an alert
   */
  async resolveAlert(alertId: UUID, resolution?: string): Promise<Alert> {
    const response = await post<Alert>(`/monitoring/alerts/${alertId}/resolve`, {
      resolution,
    });

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to resolve alert');
    }

    return response.data;
  },

  /**
   * Get blocking analysis tree
   */
  async getBlockingTree(): Promise<BlockingTree> {
    const response = await get<BlockingTree>('/monitoring/blocking');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch blocking tree');
    }

    return response.data;
  },

  /**
   * Get lock statistics
   */
  async getLockStats(): Promise<LockStats> {
    const response = await get<LockStats>('/monitoring/locks/stats');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch lock statistics');
    }

    return response.data;
  },

  /**
   * Get alert rules
   */
  async getAlertRules(): Promise<AlertRule[]> {
    const response = await get<AlertRule[]>('/monitoring/alert-rules');

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to fetch alert rules');
    }

    return response.data;
  },

  /**
   * Create or update alert rule
   */
  async saveAlertRule(rule: Partial<AlertRule>): Promise<AlertRule> {
    const endpoint = rule.id
      ? `/monitoring/alert-rules/${rule.id}`
      : '/monitoring/alert-rules';

    const response = await post<AlertRule>(endpoint, rule);

    if (!response.success || !response.data) {
      throw new Error(response.error?.message || 'Failed to save alert rule');
    }

    return response.data;
  },

  /**
   * Delete alert rule
   */
  async deleteAlertRule(ruleId: UUID): Promise<void> {
    const response = await del<void>(`/monitoring/alert-rules/${ruleId}`);

    if (!response.success) {
      throw new Error(response.error?.message || 'Failed to delete alert rule');
    }
  },

  /**
   * Create WebSocket connection for real-time monitoring updates
   */
  createWebSocket(): MonitoringWebSocket {
    let ws: WebSocket | null = null;
    const subscriptions = new Set<string>();

    return {
      connect: (onMessage: (data: unknown) => void) => {
        // Get auth token for WebSocket
        const storedAuth = localStorage.getItem('rustydb_auth');
        let token = '';
        if (storedAuth) {
          try {
            const { session } = JSON.parse(storedAuth);
            token = session?.token || '';
          } catch {
            // Invalid auth, continue without token
          }
        }

        ws = new WebSocket(`${WS_URL}/monitoring?token=${token}`);

        ws.onopen = () => {
          console.log('[Monitoring WS] Connected');
          // Re-subscribe to previous topics
          subscriptions.forEach((topic) => {
            ws?.send(JSON.stringify({ type: 'subscribe', topic }));
          });
        };

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            onMessage(data);
          } catch (error) {
            console.error('[Monitoring WS] Failed to parse message:', error);
          }
        };

        ws.onerror = (error) => {
          console.error('[Monitoring WS] Error:', error);
        };

        ws.onclose = () => {
          console.log('[Monitoring WS] Disconnected');
          // Auto-reconnect after 5 seconds
          setTimeout(() => {
            if (ws?.readyState === WebSocket.CLOSED) {
              this.connect(onMessage);
            }
          }, 5000);
        };
      },

      disconnect: () => {
        if (ws) {
          ws.close();
          ws = null;
        }
        subscriptions.clear();
      },

      subscribe: (topic: string) => {
        subscriptions.add(topic);
        if (ws && ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: 'subscribe', topic }));
        }
      },

      unsubscribe: (topic: string) => {
        subscriptions.delete(topic);
        if (ws && ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: 'unsubscribe', topic }));
        }
      },
    };
  },
};
