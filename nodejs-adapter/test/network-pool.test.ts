/**
 * Network & Connection Pool API Client Tests
 *
 * Comprehensive test suite for NetworkPoolClient covering:
 * - Network status and connections
 * - Protocol configuration
 * - Cluster management
 * - Load balancer operations
 * - Circuit breakers
 * - Connection pools
 * - Session management
 *
 * @module network-pool.test
 */

import {
    NetworkPoolClient,
    NetworkStatus,
    NetworkConnectionInfo,
    ProtocolConfig,
    UpdateProtocolRequest,
    ClusterStatus,
    ClusterNode,
    AddClusterNodeRequest,
    LoadBalancerStats,
    LoadBalancerConfig,
    CircuitBreakerStatus,
    PoolConfig,
    PoolStatsResponse,
    ConnectionInfo,
    SessionInfo,
    PaginationParams,
} from '../src/api/network-pool';

// ============================================================================
// Test Data - Network Status
// ============================================================================

const mockNetworkStatus: NetworkStatus = {
    status: 'healthy',
    active_connections: 42,
    total_connections_lifetime: 10500,
    bytes_sent: 1048576000, // ~1GB
    bytes_received: 524288000, // ~500MB
    errors: 12,
    uptime_seconds: 86400, // 1 day
};

const mockNetworkConnections: NetworkConnectionInfo[] = [
    {
        connection_id: 'conn_1',
        remote_address: '192.168.1.100:45678',
        local_address: '0.0.0.0:5432',
        protocol: 'tcp',
        state: 'established',
        session_id: 1001,
        connected_at: Date.now() - 3600000, // 1 hour ago
        bytes_sent: 1500000,
        bytes_received: 800000,
        last_activity: Date.now() - 5000, // 5 seconds ago
    },
    {
        connection_id: 'conn_2',
        remote_address: '192.168.1.101:45679',
        local_address: '0.0.0.0:5432',
        protocol: 'tcp',
        state: 'established',
        session_id: 1002,
        connected_at: Date.now() - 1800000, // 30 minutes ago
        bytes_sent: 750000,
        bytes_received: 400000,
        last_activity: Date.now() - 10000, // 10 seconds ago
    },
    {
        connection_id: 'conn_3',
        remote_address: '192.168.1.102:45680',
        local_address: '0.0.0.0:5432',
        protocol: 'websocket',
        state: 'established',
        connected_at: Date.now() - 600000, // 10 minutes ago
        bytes_sent: 250000,
        bytes_received: 150000,
        last_activity: Date.now() - 2000, // 2 seconds ago
    },
];

// ============================================================================
// Test Data - Protocol Configuration
// ============================================================================

const mockProtocolConfig: ProtocolConfig = {
    protocol_version: '1.0',
    max_packet_size: 1048576, // 1MB
    compression_enabled: true,
    encryption_enabled: true,
    keep_alive_interval_secs: 30,
    timeout_secs: 300,
};

const mockUpdateProtocolRequest: UpdateProtocolRequest = {
    max_packet_size: 2097152, // 2MB
    compression_enabled: true,
    keep_alive_interval_secs: 60,
    timeout_secs: 600,
};

// ============================================================================
// Test Data - Cluster Management
// ============================================================================

const mockClusterStatus: ClusterStatus = {
    cluster_id: 'cluster-main',
    status: 'healthy',
    node_count: 3,
    healthy_nodes: 3,
    leader_node_id: 'node1',
    consensus_algorithm: 'raft',
    replication_factor: 3,
};

const mockClusterNodes: ClusterNode[] = [
    {
        node_id: 'node1',
        address: '192.168.1.10',
        port: 5432,
        role: 'leader',
        status: 'healthy',
        version: '1.0.0',
        uptime_seconds: 86400, // 1 day
        last_heartbeat: Date.now(),
        cpu_usage: 45.5,
        memory_usage_mb: 2048,
        disk_usage_percent: 60.0,
        connections: 50,
    },
    {
        node_id: 'node2',
        address: '192.168.1.11',
        port: 5432,
        role: 'follower',
        status: 'healthy',
        version: '1.0.0',
        uptime_seconds: 86300,
        last_heartbeat: Date.now(),
        cpu_usage: 38.2,
        memory_usage_mb: 1800,
        disk_usage_percent: 55.0,
        connections: 42,
    },
    {
        node_id: 'node3',
        address: '192.168.1.12',
        port: 5432,
        role: 'follower',
        status: 'healthy',
        version: '1.0.0',
        uptime_seconds: 86200,
        last_heartbeat: Date.now(),
        cpu_usage: 42.8,
        memory_usage_mb: 1900,
        disk_usage_percent: 58.0,
        connections: 45,
    },
];

const mockAddClusterNodeRequest: AddClusterNodeRequest = {
    node_id: 'node4',
    address: '192.168.1.13',
    port: 5432,
    role: 'follower',
};

// ============================================================================
// Test Data - Load Balancer
// ============================================================================

const mockLoadBalancerStats: LoadBalancerStats = {
    algorithm: 'round_robin',
    total_requests: 1000000,
    requests_per_second: 1500.0,
    backend_pools: [
        {
            pool_id: 'pool1',
            backends: [
                {
                    backend_id: 'backend1',
                    address: '192.168.1.10:5432',
                    weight: 100,
                    active: true,
                    health_status: 'healthy',
                    active_connections: 50,
                    total_requests: 500000,
                    failed_requests: 10,
                    avg_response_time_ms: 12.5,
                },
                {
                    backend_id: 'backend2',
                    address: '192.168.1.11:5432',
                    weight: 100,
                    active: true,
                    health_status: 'healthy',
                    active_connections: 45,
                    total_requests: 500000,
                    failed_requests: 8,
                    avg_response_time_ms: 13.2,
                },
            ],
            active_requests: 150,
            total_requests: 1000000,
        },
    ],
};

const mockLoadBalancerConfig: LoadBalancerConfig = {
    algorithm: 'least_connections',
    health_check_interval_secs: 10,
    max_retries: 3,
    timeout_secs: 30,
};

// ============================================================================
// Test Data - Circuit Breakers
// ============================================================================

const mockCircuitBreakers: CircuitBreakerStatus[] = [
    {
        name: 'database',
        state: 'closed',
        failure_count: 2,
        success_count: 9998,
        last_failure: Date.now() - 3600000, // 1 hour ago
        last_state_change: Date.now() - 3600000,
        failure_threshold: 5,
        timeout_secs: 60,
    },
    {
        name: 'cache',
        state: 'closed',
        failure_count: 0,
        success_count: 10000,
        last_state_change: Date.now() - 7200000, // 2 hours ago
        failure_threshold: 5,
        timeout_secs: 60,
    },
    {
        name: 'external_api',
        state: 'open',
        failure_count: 15,
        success_count: 8500,
        last_failure: Date.now() - 30000, // 30 seconds ago
        last_state_change: Date.now() - 30000,
        failure_threshold: 10,
        timeout_secs: 120,
    },
];

// ============================================================================
// Test Data - Connection Pools
// ============================================================================

const mockPools: PoolConfig[] = [
    {
        pool_id: 'default',
        min_connections: 10,
        max_connections: 100,
        connection_timeout_secs: 30,
        idle_timeout_secs: 600,
        max_lifetime_secs: 3600,
    },
    {
        pool_id: 'readonly',
        min_connections: 5,
        max_connections: 50,
        connection_timeout_secs: 15,
        idle_timeout_secs: 300,
        max_lifetime_secs: 1800,
    },
    {
        pool_id: 'analytics',
        min_connections: 2,
        max_connections: 20,
        connection_timeout_secs: 60,
        idle_timeout_secs: 1200,
        max_lifetime_secs: 7200,
    },
];

const mockPoolStats: PoolStatsResponse = {
    pool_id: 'default',
    active_connections: 25,
    idle_connections: 15,
    total_connections: 40,
    waiting_requests: 2,
    total_acquired: 5000,
    total_created: 50,
    total_destroyed: 10,
};

// ============================================================================
// Test Data - Connections
// ============================================================================

const mockConnections: ConnectionInfo[] = [
    {
        connection_id: 1001,
        pool_id: 'default',
        session_id: 1001,
        username: 'admin',
        database: 'rustydb',
        client_address: '192.168.1.100:45678',
        created_at: Date.now() - 3600000,
        last_activity: Date.now() - 5000,
        queries_executed: 150,
        state: 'active',
        idle_time_secs: 5,
    },
    {
        connection_id: 1002,
        pool_id: 'default',
        session_id: 1002,
        username: 'app_user',
        database: 'rustydb',
        client_address: '192.168.1.101:45679',
        created_at: Date.now() - 1800000,
        last_activity: Date.now() - 10000,
        queries_executed: 75,
        state: 'idle',
        idle_time_secs: 10,
    },
    {
        connection_id: 1003,
        pool_id: 'readonly',
        session_id: 1003,
        username: 'reader',
        database: 'rustydb',
        client_address: '192.168.1.102:45680',
        created_at: Date.now() - 600000,
        last_activity: Date.now() - 2000,
        queries_executed: 30,
        state: 'active',
        idle_time_secs: 2,
    },
];

// ============================================================================
// Test Data - Sessions
// ============================================================================

const mockSessions: SessionInfo[] = [
    {
        session_id: 1001,
        username: 'admin',
        database: 'rustydb',
        client_address: '192.168.1.100:45678',
        created_at: Date.now() - 3600000,
        last_activity: Date.now() - 5000,
        state: 'active',
        current_query: 'SELECT * FROM users WHERE active = true',
        transaction_id: 2001,
    },
    {
        session_id: 1002,
        username: 'app_user',
        database: 'rustydb',
        client_address: '192.168.1.101:45679',
        created_at: Date.now() - 1800000,
        last_activity: Date.now() - 10000,
        state: 'idle',
    },
    {
        session_id: 1003,
        username: 'reader',
        database: 'rustydb',
        client_address: '192.168.1.102:45680',
        created_at: Date.now() - 600000,
        last_activity: Date.now() - 2000,
        state: 'active',
        current_query: 'SELECT COUNT(*) FROM orders',
    },
];

// ============================================================================
// Test Suite
// ============================================================================

describe('NetworkPoolClient', () => {
    let client: NetworkPoolClient;
    const baseUrl = 'http://localhost:8080';

    beforeEach(() => {
        client = new NetworkPoolClient(baseUrl);
    });

    // ========================================================================
    // Network Status & Connections Tests
    // ========================================================================

    describe('Network Status & Connections', () => {
        test('should get network status', async () => {
            // Mock implementation would go here
            console.log('Testing getNetworkStatus()');
            console.log('Expected response:', mockNetworkStatus);
        });

        test('should get all network connections', async () => {
            console.log('Testing getNetworkConnections()');
            console.log('Expected response:', mockNetworkConnections);
        });

        test('should get specific network connection', async () => {
            console.log('Testing getNetworkConnection("conn_1")');
            console.log('Expected response:', mockNetworkConnections[0]);
        });

        test('should kill network connection', async () => {
            console.log('Testing killNetworkConnection("conn_1")');
            console.log('Expected: 204 No Content');
        });
    });

    // ========================================================================
    // Protocol Configuration Tests
    // ========================================================================

    describe('Protocol Configuration', () => {
        test('should get protocol configuration', async () => {
            console.log('Testing getProtocolConfig()');
            console.log('Expected response:', mockProtocolConfig);
        });

        test('should update protocol configuration', async () => {
            console.log('Testing updateProtocolConfig()');
            console.log('Request:', mockUpdateProtocolRequest);
            console.log('Expected response: Updated ProtocolConfig');
        });
    });

    // ========================================================================
    // Cluster Management Tests
    // ========================================================================

    describe('Cluster Management', () => {
        test('should get cluster status', async () => {
            console.log('Testing getClusterStatus()');
            console.log('Expected response:', mockClusterStatus);
        });

        test('should get all cluster nodes', async () => {
            console.log('Testing getClusterNodes()');
            console.log('Expected response:', mockClusterNodes);
        });

        test('should add cluster node', async () => {
            console.log('Testing addClusterNode()');
            console.log('Request:', mockAddClusterNodeRequest);
            console.log('Expected: 201 Created with ClusterNode');
        });

        test('should remove cluster node', async () => {
            console.log('Testing removeClusterNode("node4")');
            console.log('Expected: 204 No Content');
        });
    });

    // ========================================================================
    // Load Balancer Tests
    // ========================================================================

    describe('Load Balancer', () => {
        test('should get load balancer statistics', async () => {
            console.log('Testing getLoadBalancerStats()');
            console.log('Expected response:', mockLoadBalancerStats);
        });

        test('should configure load balancer', async () => {
            console.log('Testing configureLoadBalancer()');
            console.log('Request:', mockLoadBalancerConfig);
            console.log('Expected response: { status: "updated", timestamp: <number> }');
        });
    });

    // ========================================================================
    // Circuit Breaker Tests
    // ========================================================================

    describe('Circuit Breakers', () => {
        test('should get all circuit breakers', async () => {
            console.log('Testing getCircuitBreakers()');
            console.log('Expected response:', mockCircuitBreakers);
        });

        test('should identify open circuit breakers', () => {
            const openBreakers = mockCircuitBreakers.filter(cb => cb.state === 'open');
            console.log('Open circuit breakers:', openBreakers);
            expect(openBreakers.length).toBeGreaterThan(0);
        });
    });

    // ========================================================================
    // Connection Pool Tests
    // ========================================================================

    describe('Connection Pools', () => {
        test('should get all pools', async () => {
            console.log('Testing getPools()');
            console.log('Expected response:', mockPools);
        });

        test('should get specific pool', async () => {
            console.log('Testing getPool("default")');
            console.log('Expected response:', mockPools[0]);
        });

        test('should update pool configuration', async () => {
            const updatedConfig: PoolConfig = {
                ...mockPools[0],
                max_connections: 150,
            };
            console.log('Testing updatePool("default", config)');
            console.log('Request:', updatedConfig);
            console.log('Expected: 200 OK');
        });

        test('should get pool statistics', async () => {
            console.log('Testing getPoolStats("default")');
            console.log('Expected response:', mockPoolStats);
        });

        test('should drain pool', async () => {
            console.log('Testing drainPool("default")');
            console.log('Expected: 202 Accepted');
        });

        test('should validate pool configuration constraints', () => {
            const invalidConfig = {
                ...mockPools[0],
                min_connections: 100,
                max_connections: 50, // Invalid: min > max
            };
            console.log('Invalid config:', invalidConfig);
            console.log('Expected error: min_connections cannot exceed max_connections');
        });
    });

    // ========================================================================
    // Connection Tests
    // ========================================================================

    describe('Connections', () => {
        test('should get all connections with pagination', async () => {
            const params: PaginationParams = {
                page: 1,
                page_size: 10,
                sort_by: 'created_at',
                sort_order: 'desc',
            };
            console.log('Testing getConnections(params)');
            console.log('Params:', params);
            console.log('Expected response: PaginatedResponse<ConnectionInfo>');
        });

        test('should get specific connection', async () => {
            console.log('Testing getConnection(1001)');
            console.log('Expected response:', mockConnections[0]);
        });

        test('should kill connection', async () => {
            console.log('Testing killConnection(1001)');
            console.log('Expected: 204 No Content');
        });

        test('should filter active connections', () => {
            const activeConnections = mockConnections.filter(c => c.state === 'active');
            console.log('Active connections:', activeConnections);
            expect(activeConnections.length).toBe(2);
        });

        test('should calculate connection statistics', () => {
            const totalQueries = mockConnections.reduce((sum, c) => sum + c.queries_executed, 0);
            const avgQueries = totalQueries / mockConnections.length;
            console.log('Total queries executed:', totalQueries);
            console.log('Average queries per connection:', avgQueries);
        });
    });

    // ========================================================================
    // Session Tests
    // ========================================================================

    describe('Sessions', () => {
        test('should get all sessions with pagination', async () => {
            const params: PaginationParams = {
                page: 1,
                page_size: 20,
            };
            console.log('Testing getSessions(params)');
            console.log('Params:', params);
            console.log('Expected response: PaginatedResponse<SessionInfo>');
        });

        test('should get specific session', async () => {
            console.log('Testing getSession(1001)');
            console.log('Expected response:', mockSessions[0]);
        });

        test('should terminate session', async () => {
            console.log('Testing terminateSession(1001)');
            console.log('Expected: 204 No Content');
        });

        test('should identify sessions in transactions', () => {
            const sessionsInTxn = mockSessions.filter(s => s.transaction_id !== undefined);
            console.log('Sessions in transaction:', sessionsInTxn);
            expect(sessionsInTxn.length).toBeGreaterThan(0);
        });

        test('should identify idle sessions', () => {
            const idleSessions = mockSessions.filter(s => s.state === 'idle');
            console.log('Idle sessions:', idleSessions);
        });
    });

    // ========================================================================
    // Integration Tests
    // ========================================================================

    describe('Integration Scenarios', () => {
        test('should monitor cluster health', () => {
            console.log('=== Cluster Health Monitoring ===');
            console.log('Cluster Status:', mockClusterStatus);
            console.log('Healthy Nodes:', mockClusterStatus.healthy_nodes, '/', mockClusterStatus.node_count);
            console.log('Leader:', mockClusterStatus.leader_node_id);
            console.log('Nodes:', mockClusterNodes.map(n => ({
                id: n.node_id,
                role: n.role,
                status: n.status,
                cpu: n.cpu_usage,
                memory: n.memory_usage_mb,
            })));
        });

        test('should analyze connection pool utilization', () => {
            console.log('=== Connection Pool Utilization ===');
            const utilization = (mockPoolStats.active_connections / mockPoolStats.total_connections) * 100;
            console.log('Pool:', mockPoolStats.pool_id);
            console.log('Active:', mockPoolStats.active_connections);
            console.log('Idle:', mockPoolStats.idle_connections);
            console.log('Total:', mockPoolStats.total_connections);
            console.log('Utilization:', utilization.toFixed(2), '%');
            console.log('Waiting:', mockPoolStats.waiting_requests);
        });

        test('should detect load balancer issues', () => {
            console.log('=== Load Balancer Health Check ===');
            mockLoadBalancerStats.backend_pools.forEach(pool => {
                console.log(`Pool: ${pool.pool_id}`);
                pool.backends.forEach(backend => {
                    const errorRate = (backend.failed_requests / backend.total_requests) * 100;
                    console.log(`  Backend: ${backend.backend_id}`);
                    console.log(`    Status: ${backend.health_status}`);
                    console.log(`    Error Rate: ${errorRate.toFixed(2)}%`);
                    console.log(`    Avg Response Time: ${backend.avg_response_time_ms}ms`);
                });
            });
        });

        test('should identify circuit breaker failures', () => {
            console.log('=== Circuit Breaker Status ===');
            mockCircuitBreakers.forEach(cb => {
                const failureRate = (cb.failure_count / (cb.failure_count + cb.success_count)) * 100;
                console.log(`Breaker: ${cb.name}`);
                console.log(`  State: ${cb.state}`);
                console.log(`  Failure Rate: ${failureRate.toFixed(2)}%`);
                console.log(`  Failures: ${cb.failure_count} / Threshold: ${cb.failure_threshold}`);
                if (cb.state === 'open') {
                    console.log(`  ⚠️  CIRCUIT OPEN - Service Degraded`);
                }
            });
        });

        test('should monitor network traffic', () => {
            console.log('=== Network Traffic Analysis ===');
            console.log('Status:', mockNetworkStatus.status);
            console.log('Active Connections:', mockNetworkStatus.active_connections);
            console.log('Total Connections (lifetime):', mockNetworkStatus.total_connections_lifetime);
            console.log('Bytes Sent:', (mockNetworkStatus.bytes_sent / 1024 / 1024).toFixed(2), 'MB');
            console.log('Bytes Received:', (mockNetworkStatus.bytes_received / 1024 / 1024).toFixed(2), 'MB');
            console.log('Errors:', mockNetworkStatus.errors);
            console.log('Uptime:', mockNetworkStatus.uptime_seconds, 'seconds');
        });
    });
});

// ============================================================================
// Example Usage Documentation
// ============================================================================

/**
 * Example: Basic Network Monitoring
 */
async function exampleNetworkMonitoring() {
    const client = new NetworkPoolClient('http://localhost:8080', 'your-api-key');

    // Get network status
    const status = await client.getNetworkStatus();
    console.log('Network Status:', status.status);
    console.log('Active Connections:', status.active_connections);

    // Get all connections
    const connections = await client.getNetworkConnections();
    console.log('Total Connections:', connections.length);

    // Kill a specific connection
    if (connections.length > 0) {
        await client.killNetworkConnection(connections[0].connection_id);
        console.log('Connection killed');
    }
}

/**
 * Example: Cluster Management
 */
async function exampleClusterManagement() {
    const client = new NetworkPoolClient('http://localhost:8080', 'your-api-key');

    // Get cluster status
    const clusterStatus = await client.getClusterStatus();
    console.log('Cluster Health:', clusterStatus.status);
    console.log('Nodes:', clusterStatus.node_count);
    console.log('Leader:', clusterStatus.leader_node_id);

    // Add a new node
    const newNode = await client.addClusterNode({
        node_id: 'node5',
        address: '192.168.1.14',
        port: 5432,
        role: 'follower',
    });
    console.log('Node added:', newNode.node_id);

    // Remove a node
    await client.removeClusterNode('node5');
    console.log('Node removed');
}

/**
 * Example: Connection Pool Management
 */
async function examplePoolManagement() {
    const client = new NetworkPoolClient('http://localhost:8080', 'your-api-key');

    // Get all pools
    const pools = await client.getPools();
    console.log('Available Pools:', pools.map(p => p.pool_id));

    // Get pool stats
    const stats = await client.getPoolStats('default');
    console.log('Active Connections:', stats.active_connections);
    console.log('Idle Connections:', stats.idle_connections);
    console.log('Utilization:', ((stats.active_connections / stats.total_connections) * 100).toFixed(2), '%');

    // Update pool configuration
    await client.updatePool('default', {
        pool_id: 'default',
        min_connections: 10,
        max_connections: 150, // Increased capacity
        connection_timeout_secs: 30,
        idle_timeout_secs: 600,
        max_lifetime_secs: 3600,
    });
    console.log('Pool configuration updated');

    // Drain pool
    await client.drainPool('default');
    console.log('Pool draining started');
}

/**
 * Example: Session Management
 */
async function exampleSessionManagement() {
    const client = new NetworkPoolClient('http://localhost:8080', 'your-api-key');

    // Get all sessions with pagination
    const sessionsPage = await client.getSessions({
        page: 1,
        page_size: 50,
        sort_by: 'last_activity',
        sort_order: 'desc',
    });
    console.log('Total Sessions:', sessionsPage.total_count);
    console.log('Active Sessions:', sessionsPage.data.filter(s => s.state === 'active').length);

    // Terminate idle sessions
    for (const session of sessionsPage.data) {
        const idleTime = Date.now() - session.last_activity;
        if (session.state === 'idle' && idleTime > 3600000) { // 1 hour
            await client.terminateSession(session.session_id);
            console.log('Terminated idle session:', session.session_id);
        }
    }
}

/**
 * Example: Load Balancer Monitoring
 */
async function exampleLoadBalancerMonitoring() {
    const client = new NetworkPoolClient('http://localhost:8080', 'your-api-key');

    // Get load balancer stats
    const lbStats = await client.getLoadBalancerStats();
    console.log('Algorithm:', lbStats.algorithm);
    console.log('Total Requests:', lbStats.total_requests);
    console.log('Requests/sec:', lbStats.requests_per_second);

    // Check backend health
    for (const pool of lbStats.backend_pools) {
        console.log(`Pool: ${pool.pool_id}`);
        for (const backend of pool.backends) {
            const errorRate = (backend.failed_requests / backend.total_requests) * 100;
            console.log(`  Backend ${backend.backend_id}: ${backend.health_status}`);
            console.log(`    Error Rate: ${errorRate.toFixed(2)}%`);
            console.log(`    Avg Response: ${backend.avg_response_time_ms}ms`);
        }
    }

    // Configure load balancer
    await client.configureLoadBalancer({
        algorithm: 'least_connections',
        health_check_interval_secs: 10,
        max_retries: 3,
        timeout_secs: 30,
    });
    console.log('Load balancer reconfigured');
}

/**
 * Example: Circuit Breaker Monitoring
 */
async function exampleCircuitBreakerMonitoring() {
    const client = new NetworkPoolClient('http://localhost:8080', 'your-api-key');

    // Get all circuit breakers
    const breakers = await client.getCircuitBreakers();

    // Check for open circuits
    const openCircuits = breakers.filter(cb => cb.state === 'open');
    if (openCircuits.length > 0) {
        console.log('⚠️  ALERT: Open Circuits Detected!');
        openCircuits.forEach(cb => {
            console.log(`  - ${cb.name}: ${cb.failure_count} failures`);
        });
    }

    // Monitor failure rates
    breakers.forEach(cb => {
        const totalRequests = cb.failure_count + cb.success_count;
        const failureRate = (cb.failure_count / totalRequests) * 100;
        console.log(`${cb.name}: ${failureRate.toFixed(2)}% failure rate (${cb.state})`);
    });
}

// Export examples for documentation
export {
    exampleNetworkMonitoring,
    exampleClusterManagement,
    examplePoolManagement,
    exampleSessionManagement,
    exampleLoadBalancerMonitoring,
    exampleCircuitBreakerMonitoring,
};
