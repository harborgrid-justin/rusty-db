/**
 * RustyDB Node.js Adapter - Monitoring & Health API
 *
 * Comprehensive TypeScript adapter for all monitoring, health, diagnostics,
 * and observability endpoints in RustyDB.
 *
 * Endpoints covered:
 * - Health probes (liveness, readiness, startup)
 * - Metrics collection and Prometheus export
 * - Session and query statistics
 * - Performance monitoring
 * - Diagnostics and profiling
 * - Active Session History (ASH)
 * - Logs and alerts
 */

import axios, { AxiosInstance, AxiosRequestConfig } from 'axios';

// ============================================================================
// Type Definitions - Health
// ============================================================================

export interface LivenessProbeResponse {
  status: string;
  timestamp: number;
  uptime_seconds?: number;
}

export interface ReadinessProbeResponse {
  status: string;
  timestamp: number;
  ready: boolean;
  dependencies: Record<string, string>;
}

export interface StartupProbeResponse {
  status: string;
  timestamp: number;
  initialized: boolean;
  checks: Record<string, boolean>;
}

export interface ComponentHealthDetail {
  component: string;
  status: string;
  message: string;
  duration_ms: number;
  details: Record<string, any>;
}

export interface FullHealthResponse {
  status: string;
  timestamp: number;
  components: ComponentHealthDetail[];
  liveness: LivenessProbeResponse;
  readiness: ReadinessProbeResponse;
  startup: StartupProbeResponse;
}

// ============================================================================
// Type Definitions - Metrics
// ============================================================================

export interface MetricData {
  value: number;
  unit: string;
  labels: Record<string, string>;
}

export interface MetricsResponse {
  timestamp: number;
  metrics: Record<string, MetricData>;
  prometheus_format?: string;
}

// ============================================================================
// Type Definitions - Session & Query Statistics
// ============================================================================

export interface SessionInfo {
  session_id: { 0: number };
  username: string;
  database: string;
  client_address?: string;
  created_at: number;
  last_activity: number;
  state: string;
  current_query?: string;
  transaction_id?: { 0: number };
}

export interface SessionStatsResponse {
  active_sessions: number;
  idle_sessions: number;
  sessions: SessionInfo[];
  total_connections: number;
  peak_connections: number;
}

export interface SlowQueryInfo {
  query: string;
  execution_time_ms: number;
  timestamp: number;
  user: string;
}

export interface TopQueryInfo {
  query_pattern: string;
  execution_count: number;
  total_time_ms: number;
  avg_time_ms: number;
}

export interface QueryStatsResponse {
  total_queries: number;
  queries_per_second: number;
  avg_execution_time_ms: number;
  slow_queries: SlowQueryInfo[];
  top_queries: TopQueryInfo[];
}

// ============================================================================
// Type Definitions - Performance
// ============================================================================

export interface PerformanceDataResponse {
  cpu_usage_percent: number;
  memory_usage_bytes: number;
  memory_usage_percent: number;
  disk_io_read_bytes: number;
  disk_io_write_bytes: number;
  cache_hit_ratio: number;
  transactions_per_second: number;
  locks_held: number;
  deadlocks: number;
}

// ============================================================================
// Type Definitions - Diagnostics
// ============================================================================

export interface IncidentSummary {
  id: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  status: 'open' | 'investigating' | 'resolved' | 'closed';
  title: string;
  description: string;
  created_at: number;
  updated_at: number;
  resolved_at?: number;
  affected_components: string[];
  assignee?: string;
}

export interface IncidentListResponse {
  incidents: IncidentSummary[];
  total_count: number;
  page: number;
  page_size: number;
}

export interface DumpRequest {
  dump_type: 'memory' | 'thread' | 'heap' | 'query_plan' | 'execution_stats';
  target?: string;
  include_stacktrace?: boolean;
  format?: 'json' | 'text' | 'binary';
}

export interface DumpResponse {
  dump_id: string;
  dump_type: string;
  status: 'pending' | 'in_progress' | 'completed' | 'failed';
  created_at: number;
  completed_at?: number;
  size_bytes?: number;
  download_url?: string;
  expires_at?: number;
}

// ============================================================================
// Type Definitions - Profiling
// ============================================================================

export interface QueryProfile {
  query_id: string;
  query_text: string;
  execution_count: number;
  total_time_ms: number;
  avg_time_ms: number;
  min_time_ms: number;
  max_time_ms: number;
  total_rows_returned: number;
  avg_rows_returned: number;
  cache_hit_ratio: number;
  last_executed: number;
  execution_plan?: string;
}

export interface QueryProfilingResponse {
  profiles: QueryProfile[];
  total_count: number;
  page: number;
  page_size: number;
}

// ============================================================================
// Type Definitions - Active Session History (ASH)
// ============================================================================

export interface TimeRange {
  start: number;
  end: number;
}

export interface ASHSample {
  sample_time: number;
  session_id: number;
  sql_id?: string;
  sql_text?: string;
  wait_event?: string;
  wait_time_ms?: number;
  blocking_session?: number;
  cpu_time_ms: number;
  user_name: string;
  program?: string;
  module?: string;
  action?: string;
}

export interface ActiveSessionHistoryResponse {
  samples: ASHSample[];
  total_count: number;
  sample_interval_seconds: number;
  time_range: TimeRange;
}

// ============================================================================
// Type Definitions - Logs & Alerts
// ============================================================================

export interface LogEntry {
  timestamp: number;
  level: string;
  message: string;
  context: Record<string, any>;
}

export interface LogResponse {
  entries: LogEntry[];
  total_count: number;
  has_more: boolean;
}

export interface Alert {
  alert_id: string;
  severity: string;
  title: string;
  description: string;
  triggered_at: number;
  acknowledged: boolean;
}

export interface AlertResponse {
  alerts: Alert[];
  active_count: number;
}

// ============================================================================
// Query Parameters
// ============================================================================

export interface PaginationParams {
  page?: number;
  page_size?: number;
  sort_by?: string;
  sort_order?: 'asc' | 'desc';
}

export interface IncidentFilterParams extends PaginationParams {
  severity?: 'critical' | 'high' | 'medium' | 'low';
  status?: 'open' | 'investigating' | 'resolved' | 'closed';
}

export interface ProfilingQueryParams extends PaginationParams {
  min_time_ms?: number;
}

export interface ASHQueryParams {
  start_time?: number;
  end_time?: number;
  session_id?: number;
  wait_event?: string;
}

// ============================================================================
// Client Configuration
// ============================================================================

export interface MonitoringClientConfig {
  baseURL: string;
  timeout?: number;
  headers?: Record<string, string>;
  apiKey?: string;
}

// ============================================================================
// Monitoring Client Class
// ============================================================================

export class RustyDBMonitoringClient {
  private client: AxiosInstance;

  constructor(config: MonitoringClientConfig) {
    const headers: Record<string, string> = config.headers || {};

    if (config.apiKey) {
      headers['X-API-Key'] = config.apiKey;
    }

    this.client = axios.create({
      baseURL: config.baseURL,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        ...headers,
      },
    });
  }

  // ========================================================================
  // Health Probe Methods
  // ========================================================================

  /**
   * Liveness probe - indicates if the service is alive
   * @returns Promise<LivenessProbeResponse>
   */
  async getLivenessProbe(): Promise<LivenessProbeResponse> {
    const response = await this.client.get<LivenessProbeResponse>('/api/v1/health/liveness');
    return response.data;
  }

  /**
   * Readiness probe - indicates if the service is ready to accept traffic
   * @returns Promise<ReadinessProbeResponse>
   */
  async getReadinessProbe(): Promise<ReadinessProbeResponse> {
    const response = await this.client.get<ReadinessProbeResponse>('/api/v1/health/readiness');
    return response.data;
  }

  /**
   * Startup probe - indicates if the service has completed initialization
   * @returns Promise<StartupProbeResponse>
   */
  async getStartupProbe(): Promise<StartupProbeResponse> {
    const response = await this.client.get<StartupProbeResponse>('/api/v1/health/startup');
    return response.data;
  }

  /**
   * Full health check - comprehensive health information
   * @returns Promise<FullHealthResponse>
   */
  async getFullHealthCheck(): Promise<FullHealthResponse> {
    const response = await this.client.get<FullHealthResponse>('/api/v1/health/full');
    return response.data;
  }

  // ========================================================================
  // Metrics Methods
  // ========================================================================

  /**
   * Get metrics in JSON format
   * @returns Promise<MetricsResponse>
   */
  async getMetrics(): Promise<MetricsResponse> {
    const response = await this.client.get<MetricsResponse>('/api/v1/metrics');
    return response.data;
  }

  /**
   * Get metrics in Prometheus text format
   * @returns Promise<string> Prometheus-formatted metrics
   */
  async getPrometheusMetrics(): Promise<string> {
    const response = await this.client.get<string>('/api/v1/metrics/prometheus');
    return response.data;
  }

  // ========================================================================
  // Statistics Methods
  // ========================================================================

  /**
   * Get session statistics
   * @returns Promise<SessionStatsResponse>
   */
  async getSessionStats(): Promise<SessionStatsResponse> {
    const response = await this.client.get<SessionStatsResponse>('/api/v1/stats/sessions');
    return response.data;
  }

  /**
   * Get query statistics
   * @returns Promise<QueryStatsResponse>
   */
  async getQueryStats(): Promise<QueryStatsResponse> {
    const response = await this.client.get<QueryStatsResponse>('/api/v1/stats/queries');
    return response.data;
  }

  /**
   * Get performance data
   * @returns Promise<PerformanceDataResponse>
   */
  async getPerformanceData(): Promise<PerformanceDataResponse> {
    const response = await this.client.get<PerformanceDataResponse>('/api/v1/stats/performance');
    return response.data;
  }

  // ========================================================================
  // Diagnostics Methods
  // ========================================================================

  /**
   * Get list of incidents
   * @param params Optional filter and pagination parameters
   * @returns Promise<IncidentListResponse>
   */
  async getIncidents(params?: IncidentFilterParams): Promise<IncidentListResponse> {
    const response = await this.client.get<IncidentListResponse>('/api/v1/diagnostics/incidents', { params });
    return response.data;
  }

  /**
   * Create a diagnostic dump
   * @param request Dump request parameters
   * @returns Promise<DumpResponse>
   */
  async createDump(request: DumpRequest): Promise<DumpResponse> {
    const response = await this.client.post<DumpResponse>('/api/v1/diagnostics/dump', request);
    return response.data;
  }

  /**
   * Get dump status by ID
   * @param dumpId Dump ID
   * @returns Promise<DumpResponse>
   */
  async getDumpStatus(dumpId: string): Promise<DumpResponse> {
    const response = await this.client.get<DumpResponse>(`/api/v1/diagnostics/dump/${dumpId}`);
    return response.data;
  }

  /**
   * Download a diagnostic dump
   * @param dumpId Dump ID
   * @returns Promise<ArrayBuffer> Raw dump data
   */
  async downloadDump(dumpId: string): Promise<ArrayBuffer> {
    const response = await this.client.get<ArrayBuffer>(
      `/api/v1/diagnostics/dump/${dumpId}/download`,
      { responseType: 'arraybuffer' }
    );
    return response.data;
  }

  // ========================================================================
  // Profiling Methods
  // ========================================================================

  /**
   * Get query profiling data
   * @param params Optional query parameters
   * @returns Promise<QueryProfilingResponse>
   */
  async getQueryProfiling(params?: ProfilingQueryParams): Promise<QueryProfilingResponse> {
    const response = await this.client.get<QueryProfilingResponse>('/api/v1/profiling/queries', { params });
    return response.data;
  }

  // ========================================================================
  // Active Session History (ASH) Methods
  // ========================================================================

  /**
   * Get Active Session History samples
   * @param params Optional query parameters
   * @returns Promise<ActiveSessionHistoryResponse>
   */
  async getActiveSessionHistory(params?: ASHQueryParams): Promise<ActiveSessionHistoryResponse> {
    const response = await this.client.get<ActiveSessionHistoryResponse>('/api/v1/monitoring/ash', { params });
    return response.data;
  }

  // ========================================================================
  // Logs & Alerts Methods
  // ========================================================================

  /**
   * Get log entries
   * @param params Optional pagination parameters
   * @returns Promise<LogResponse>
   */
  async getLogs(params?: PaginationParams): Promise<LogResponse> {
    const response = await this.client.get<LogResponse>('/api/v1/logs', { params });
    return response.data;
  }

  /**
   * Get active alerts
   * @returns Promise<AlertResponse>
   */
  async getAlerts(): Promise<AlertResponse> {
    const response = await this.client.get<AlertResponse>('/api/v1/alerts');
    return response.data;
  }

  /**
   * Acknowledge an alert
   * @param alertId Alert ID
   * @returns Promise<void>
   */
  async acknowledgeAlert(alertId: string): Promise<void> {
    await this.client.post(`/api/v1/alerts/${alertId}/acknowledge`);
  }

  // ========================================================================
  // Utility Methods
  // ========================================================================

  /**
   * Set custom request header
   * @param name Header name
   * @param value Header value
   */
  setHeader(name: string, value: string): void {
    this.client.defaults.headers.common[name] = value;
  }

  /**
   * Remove custom request header
   * @param name Header name
   */
  removeHeader(name: string): void {
    delete this.client.defaults.headers.common[name];
  }

  /**
   * Update API key
   * @param apiKey New API key
   */
  setApiKey(apiKey: string): void {
    this.setHeader('X-API-Key', apiKey);
  }
}

// ============================================================================
// Convenience Factory Function
// ============================================================================

/**
 * Create a new RustyDB Monitoring Client
 * @param config Client configuration
 * @returns RustyDBMonitoringClient instance
 */
export function createMonitoringClient(config: MonitoringClientConfig): RustyDBMonitoringClient {
  return new RustyDBMonitoringClient(config);
}

// ============================================================================
// Default Export
// ============================================================================

export default RustyDBMonitoringClient;
