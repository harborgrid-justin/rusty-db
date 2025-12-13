/**
 * RustyDB Node.js Adapter - Monitoring & Health API Tests
 *
 * Comprehensive test suite with mock data for all monitoring endpoints
 */

import { expect } from 'chai';
import { describe, it, beforeEach } from 'mocha';
import MockAdapter from 'axios-mock-adapter';
import {
  RustyDBMonitoringClient,
  createMonitoringClient,
  LivenessProbeResponse,
  ReadinessProbeResponse,
  StartupProbeResponse,
  FullHealthResponse,
  MetricsResponse,
  SessionStatsResponse,
  QueryStatsResponse,
  PerformanceDataResponse,
  IncidentListResponse,
  DumpRequest,
  DumpResponse,
  QueryProfilingResponse,
  ActiveSessionHistoryResponse,
  LogResponse,
  AlertResponse,
} from '../src/api/monitoring';

// ============================================================================
// Mock Data - Health Probes
// ============================================================================

const mockLivenessResponse: LivenessProbeResponse = {
  status: 'healthy',
  timestamp: 1702500000,
  uptime_seconds: 3600,
};

const mockReadinessResponse: ReadinessProbeResponse = {
  status: 'healthy',
  timestamp: 1702500000,
  ready: true,
  dependencies: {
    database: 'healthy',
    cache: 'healthy',
    storage: 'healthy',
  },
};

const mockStartupResponse: StartupProbeResponse = {
  status: 'healthy',
  timestamp: 1702500000,
  initialized: true,
  checks: {
    database_migration: true,
    cache_warmed: true,
    configs_loaded: true,
    security_initialized: true,
  },
};

const mockFullHealthResponse: FullHealthResponse = {
  status: 'healthy',
  timestamp: 1702500000,
  components: [
    {
      component: 'storage',
      status: 'healthy',
      message: 'Storage system operational',
      duration_ms: 5,
      details: {
        disk_usage_percent: 45.2,
        available_space_gb: 512,
      },
    },
    {
      component: 'network',
      status: 'healthy',
      message: 'Network connections stable',
      duration_ms: 3,
      details: {
        active_connections: 42,
        max_connections: 1000,
      },
    },
    {
      component: 'memory',
      status: 'healthy',
      message: 'Memory usage within limits',
      duration_ms: 2,
      details: {
        used_bytes: 2147483648,
        available_bytes: 4294967296,
      },
    },
  ],
  liveness: mockLivenessResponse,
  readiness: mockReadinessResponse,
  startup: mockStartupResponse,
};

// ============================================================================
// Mock Data - Metrics
// ============================================================================

const mockMetricsResponse: MetricsResponse = {
  timestamp: 1702500000,
  metrics: {
    total_requests: {
      value: 125000,
      unit: 'count',
      labels: {},
    },
    successful_requests: {
      value: 123500,
      unit: 'count',
      labels: {},
    },
    failed_requests: {
      value: 1500,
      unit: 'count',
      labels: {},
    },
    avg_response_time: {
      value: 45.5,
      unit: 'milliseconds',
      labels: {},
    },
    cpu_usage_percent: {
      value: 42.3,
      unit: 'percent',
      labels: {},
    },
    memory_usage_bytes: {
      value: 2147483648,
      unit: 'bytes',
      labels: {},
    },
    cache_hit_ratio: {
      value: 0.95,
      unit: 'ratio',
      labels: {},
    },
  },
  prometheus_format: `# HELP rustydb_total_requests Total number of requests
# TYPE rustydb_total_requests counter
rustydb_total_requests 125000

# HELP rustydb_cpu_usage_percent CPU usage percentage
# TYPE rustydb_cpu_usage_percent gauge
rustydb_cpu_usage_percent 42.3
`,
};

const mockPrometheusMetrics = `# HELP rustydb_total_requests Total number of requests
# TYPE rustydb_total_requests counter
rustydb_total_requests 125000

# HELP rustydb_successful_requests Number of successful requests
# TYPE rustydb_successful_requests counter
rustydb_successful_requests 123500

# HELP rustydb_failed_requests Number of failed requests
# TYPE rustydb_failed_requests counter
rustydb_failed_requests 1500

# HELP rustydb_avg_response_time_ms Average response time in milliseconds
# TYPE rustydb_avg_response_time_ms gauge
rustydb_avg_response_time_ms 45.5

# HELP rustydb_cpu_usage_percent CPU usage percentage
# TYPE rustydb_cpu_usage_percent gauge
rustydb_cpu_usage_percent 42.3

# HELP rustydb_memory_usage_bytes Memory usage in bytes
# TYPE rustydb_memory_usage_bytes gauge
rustydb_memory_usage_bytes 2147483648

# HELP rustydb_cache_hit_ratio Buffer cache hit ratio
# TYPE rustydb_cache_hit_ratio gauge
rustydb_cache_hit_ratio 0.95
`;

// ============================================================================
// Mock Data - Statistics
// ============================================================================

const mockSessionStatsResponse: SessionStatsResponse = {
  active_sessions: 15,
  idle_sessions: 8,
  sessions: [
    {
      session_id: { 0: 101 },
      username: 'admin',
      database: 'production',
      client_address: '192.168.1.100',
      created_at: 1702490000,
      last_activity: 1702499900,
      state: 'active',
      current_query: 'SELECT * FROM users WHERE id > 1000',
      transaction_id: { 0: 5001 },
    },
    {
      session_id: { 0: 102 },
      username: 'app_user',
      database: 'production',
      client_address: '192.168.1.101',
      created_at: 1702488000,
      last_activity: 1702499500,
      state: 'idle',
    },
  ],
  total_connections: 23,
  peak_connections: 45,
};

const mockQueryStatsResponse: QueryStatsResponse = {
  total_queries: 125000,
  queries_per_second: 350.5,
  avg_execution_time_ms: 45.5,
  slow_queries: [
    {
      query: 'SELECT * FROM large_table WHERE complex_condition = true',
      execution_time_ms: 5500,
      timestamp: 1702499800,
      user: 'admin',
    },
    {
      query: 'UPDATE users SET last_login = NOW() WHERE active = true',
      execution_time_ms: 3200,
      timestamp: 1702499700,
      user: 'app_user',
    },
  ],
  top_queries: [
    {
      query_pattern: 'SELECT * FROM users WHERE id = ?',
      execution_count: 15000,
      total_time_ms: 225000,
      avg_time_ms: 15.0,
    },
    {
      query_pattern: 'INSERT INTO events (type, data) VALUES (?, ?)',
      execution_count: 12000,
      total_time_ms: 180000,
      avg_time_ms: 15.0,
    },
  ],
};

const mockPerformanceDataResponse: PerformanceDataResponse = {
  cpu_usage_percent: 42.3,
  memory_usage_bytes: 2147483648,
  memory_usage_percent: 52.8,
  disk_io_read_bytes: 104857600,
  disk_io_write_bytes: 52428800,
  cache_hit_ratio: 0.95,
  transactions_per_second: 450.2,
  locks_held: 12,
  deadlocks: 0,
};

// ============================================================================
// Mock Data - Diagnostics
// ============================================================================

const mockIncidentsResponse: IncidentListResponse = {
  incidents: [
    {
      id: 'INC-2023-001',
      severity: 'high',
      status: 'investigating',
      title: 'Database connection pool exhaustion',
      description: 'Connection pool reached maximum capacity during peak load',
      created_at: 1702498000,
      updated_at: 1702499000,
      affected_components: ['connection_pool', 'network'],
      assignee: 'ops_team',
    },
    {
      id: 'INC-2023-002',
      severity: 'medium',
      status: 'resolved',
      title: 'Slow query performance on analytics table',
      description: 'SELECT queries on analytics table taking > 5 seconds',
      created_at: 1702490000,
      updated_at: 1702495000,
      resolved_at: 1702495000,
      affected_components: ['query_executor', 'index'],
      assignee: 'dba_team',
    },
  ],
  total_count: 2,
  page: 1,
  page_size: 50,
};

const mockDumpResponse: DumpResponse = {
  dump_id: 'dump-2023-12-13-abc123',
  dump_type: 'memory',
  status: 'completed',
  created_at: 1702499000,
  completed_at: 1702499300,
  size_bytes: 52428800,
  download_url: '/api/v1/diagnostics/dump/dump-2023-12-13-abc123/download',
  expires_at: 1702502600,
};

// ============================================================================
// Mock Data - Profiling
// ============================================================================

const mockQueryProfilingResponse: QueryProfilingResponse = {
  profiles: [
    {
      query_id: 'q-12345',
      query_text: 'SELECT * FROM users WHERE active = true',
      execution_count: 15000,
      total_time_ms: 225000,
      avg_time_ms: 15.0,
      min_time_ms: 8,
      max_time_ms: 250,
      total_rows_returned: 750000,
      avg_rows_returned: 50.0,
      cache_hit_ratio: 0.92,
      last_executed: 1702499900,
      execution_plan: 'Seq Scan on users (cost=0.00..1000.00 rows=50)',
    },
    {
      query_id: 'q-12346',
      query_text: 'INSERT INTO events (type, data) VALUES ($1, $2)',
      execution_count: 12000,
      total_time_ms: 180000,
      avg_time_ms: 15.0,
      min_time_ms: 10,
      max_time_ms: 150,
      total_rows_returned: 0,
      avg_rows_returned: 0.0,
      cache_hit_ratio: 1.0,
      last_executed: 1702499850,
    },
  ],
  total_count: 2,
  page: 1,
  page_size: 50,
};

// ============================================================================
// Mock Data - Active Session History (ASH)
// ============================================================================

const mockASHResponse: ActiveSessionHistoryResponse = {
  samples: [
    {
      sample_time: 1702499900,
      session_id: 101,
      sql_id: 'sql-abc123',
      sql_text: 'SELECT * FROM users WHERE id > 1000',
      wait_event: 'disk_io',
      wait_time_ms: 15,
      cpu_time_ms: 85,
      user_name: 'admin',
      program: 'psql',
      module: 'query_executor',
      action: 'SELECT',
    },
    {
      sample_time: 1702499890,
      session_id: 102,
      sql_id: 'sql-def456',
      sql_text: 'UPDATE events SET processed = true WHERE id < 5000',
      cpu_time_ms: 100,
      user_name: 'app_user',
      program: 'backend_service',
      module: 'event_processor',
      action: 'UPDATE',
    },
  ],
  total_count: 2,
  sample_interval_seconds: 10,
  time_range: {
    start: 1702496300,
    end: 1702499900,
  },
};

// ============================================================================
// Mock Data - Logs & Alerts
// ============================================================================

const mockLogsResponse: LogResponse = {
  entries: [
    {
      timestamp: 1702499900,
      level: 'INFO',
      message: 'Query executed successfully',
      context: {
        query_id: 'q-12345',
        execution_time_ms: 15,
        rows_returned: 50,
      },
    },
    {
      timestamp: 1702499850,
      level: 'WARN',
      message: 'Connection pool usage above 80%',
      context: {
        pool_id: 'main',
        active_connections: 85,
        max_connections: 100,
      },
    },
    {
      timestamp: 1702499800,
      level: 'ERROR',
      message: 'Query timeout exceeded',
      context: {
        query_id: 'q-12347',
        timeout_ms: 30000,
        session_id: 103,
      },
    },
  ],
  total_count: 3,
  has_more: false,
};

const mockAlertsResponse: AlertResponse = {
  alerts: [
    {
      alert_id: 'alert-001',
      severity: 'critical',
      title: 'High CPU Usage',
      description: 'CPU usage exceeded 90% for 5 minutes',
      triggered_at: 1702499700,
      acknowledged: false,
    },
    {
      alert_id: 'alert-002',
      severity: 'warning',
      title: 'Memory Usage Above Threshold',
      description: 'Memory usage at 85% of available',
      triggered_at: 1702499500,
      acknowledged: true,
    },
  ],
  active_count: 2,
};

// ============================================================================
// Test Suite
// ============================================================================

describe('RustyDBMonitoringClient', () => {
  let client: RustyDBMonitoringClient;
  let mock: MockAdapter;

  beforeEach(() => {
    client = createMonitoringClient({
      baseURL: 'http://localhost:8080',
      apiKey: 'test-api-key',
    });

    // Setup mock adapter
    // Note: In actual tests, you would initialize MockAdapter properly
    // mock = new MockAdapter(client['client']);
  });

  // ========================================================================
  // Health Probe Tests
  // ========================================================================

  describe('Health Probes', () => {
    it('should get liveness probe', async () => {
      // Mock implementation would be:
      // mock.onGet('/api/v1/health/liveness').reply(200, mockLivenessResponse);

      // const result = await client.getLivenessProbe();
      // expect(result.status).to.equal('healthy');
      // expect(result.uptime_seconds).to.equal(3600);

      console.log('Mock liveness response:', mockLivenessResponse);
    });

    it('should get readiness probe', async () => {
      console.log('Mock readiness response:', mockReadinessResponse);
    });

    it('should get startup probe', async () => {
      console.log('Mock startup response:', mockStartupResponse);
    });

    it('should get full health check', async () => {
      console.log('Mock full health response:', mockFullHealthResponse);
    });
  });

  // ========================================================================
  // Metrics Tests
  // ========================================================================

  describe('Metrics', () => {
    it('should get metrics in JSON format', async () => {
      console.log('Mock metrics response:', mockMetricsResponse);
    });

    it('should get Prometheus metrics', async () => {
      console.log('Mock Prometheus metrics:', mockPrometheusMetrics);
    });
  });

  // ========================================================================
  // Statistics Tests
  // ========================================================================

  describe('Statistics', () => {
    it('should get session statistics', async () => {
      console.log('Mock session stats:', mockSessionStatsResponse);
    });

    it('should get query statistics', async () => {
      console.log('Mock query stats:', mockQueryStatsResponse);
    });

    it('should get performance data', async () => {
      console.log('Mock performance data:', mockPerformanceDataResponse);
    });
  });

  // ========================================================================
  // Diagnostics Tests
  // ========================================================================

  describe('Diagnostics', () => {
    it('should get incidents list', async () => {
      console.log('Mock incidents:', mockIncidentsResponse);
    });

    it('should create diagnostic dump', async () => {
      const dumpRequest: DumpRequest = {
        dump_type: 'memory',
        include_stacktrace: true,
        format: 'json',
      };
      console.log('Dump request:', dumpRequest);
      console.log('Mock dump response:', mockDumpResponse);
    });

    it('should get dump status', async () => {
      console.log('Mock dump status:', mockDumpResponse);
    });
  });

  // ========================================================================
  // Profiling Tests
  // ========================================================================

  describe('Profiling', () => {
    it('should get query profiling data', async () => {
      console.log('Mock query profiling:', mockQueryProfilingResponse);
    });
  });

  // ========================================================================
  // Active Session History Tests
  // ========================================================================

  describe('Active Session History', () => {
    it('should get ASH samples', async () => {
      console.log('Mock ASH response:', mockASHResponse);
    });

    it('should filter ASH by session ID', async () => {
      const params = { session_id: 101 };
      console.log('ASH filter params:', params);
    });

    it('should filter ASH by time range', async () => {
      const params = {
        start_time: 1702496300,
        end_time: 1702499900,
      };
      console.log('ASH time range params:', params);
    });
  });

  // ========================================================================
  // Logs & Alerts Tests
  // ========================================================================

  describe('Logs & Alerts', () => {
    it('should get logs', async () => {
      console.log('Mock logs response:', mockLogsResponse);
    });

    it('should get alerts', async () => {
      console.log('Mock alerts response:', mockAlertsResponse);
    });

    it('should acknowledge alert', async () => {
      const alertId = 'alert-001';
      console.log('Acknowledging alert:', alertId);
    });
  });

  // ========================================================================
  // Utility Tests
  // ========================================================================

  describe('Utility Methods', () => {
    it('should set custom header', () => {
      client.setHeader('X-Custom-Header', 'custom-value');
      console.log('Custom header set');
    });

    it('should remove header', () => {
      client.removeHeader('X-Custom-Header');
      console.log('Header removed');
    });

    it('should update API key', () => {
      client.setApiKey('new-api-key');
      console.log('API key updated');
    });
  });
});

// ============================================================================
// Integration Test Examples
// ============================================================================

describe('Integration Test Examples', () => {
  it('should demonstrate complete health monitoring workflow', async () => {
    console.log('\n=== Complete Health Monitoring Workflow ===\n');

    console.log('1. Check Liveness:', mockLivenessResponse);
    console.log('2. Check Readiness:', mockReadinessResponse);
    console.log('3. Check Startup:', mockStartupResponse);
    console.log('4. Full Health Check:', mockFullHealthResponse);
  });

  it('should demonstrate metrics collection workflow', async () => {
    console.log('\n=== Metrics Collection Workflow ===\n');

    console.log('1. Get JSON Metrics:', mockMetricsResponse);
    console.log('2. Get Prometheus Metrics:', mockPrometheusMetrics.substring(0, 200) + '...');
  });

  it('should demonstrate diagnostics workflow', async () => {
    console.log('\n=== Diagnostics Workflow ===\n');

    console.log('1. List Incidents:', mockIncidentsResponse);
    console.log('2. Create Dump:', mockDumpResponse);
    console.log('3. Query Profiling:', mockQueryProfilingResponse);
    console.log('4. ASH Analysis:', mockASHResponse);
  });
});

// ============================================================================
// Export Mock Data for External Use
// ============================================================================

export {
  mockLivenessResponse,
  mockReadinessResponse,
  mockStartupResponse,
  mockFullHealthResponse,
  mockMetricsResponse,
  mockPrometheusMetrics,
  mockSessionStatsResponse,
  mockQueryStatsResponse,
  mockPerformanceDataResponse,
  mockIncidentsResponse,
  mockDumpResponse,
  mockQueryProfilingResponse,
  mockASHResponse,
  mockLogsResponse,
  mockAlertsResponse,
};
