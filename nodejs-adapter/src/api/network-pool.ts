/**
 * Network & Connection Pool API Client for RustyDB
 *
 * Provides TypeScript interfaces and client methods for:
 * - Network status and connection management
 * - Protocol configuration
 * - Cluster topology management
 * - Connection pool management
 * - Session management
 * - Load balancer statistics
 * - Circuit breaker monitoring
 *
 * @module network-pool
 */

// ============================================================================
// Network Interfaces
// ============================================================================

/**
 * Overall network status information
 */
export interface NetworkStatus {
    /** Current network health status: healthy, degraded, unhealthy */
    status: string;
    /** Number of currently active connections */
    active_connections: number;
    /** Total connections established over the database lifetime */
    total_connections_lifetime: number;
    /** Total bytes sent across all connections */
    bytes_sent: number;
    /** Total bytes received across all connections */
    bytes_received: number;
    /** Total network errors encountered */
    errors: number;
    /** Database uptime in seconds */
    uptime_seconds: number;
}

/**
 * Information about an active network connection
 */
export interface NetworkConnectionInfo {
    /** Unique connection identifier */
    connection_id: string;
    /** Remote client address (IP:port) */
    remote_address: string;
    /** Local server address (IP:port) */
    local_address: string;
    /** Connection protocol: tcp, websocket */
    protocol: string;
    /** Connection state: established, closing, closed */
    state: string;
    /** Associated session ID if authenticated */
    session_id?: number;
    /** Unix timestamp when connection was established */
    connected_at: number;
    /** Total bytes sent on this connection */
    bytes_sent: number;
    /** Total bytes received on this connection */
    bytes_received: number;
    /** Unix timestamp of last activity */
    last_activity: number;
}

/**
 * Protocol configuration settings
 */
export interface ProtocolConfig {
    /** Protocol version (e.g., "1.0") */
    protocol_version: string;
    /** Maximum packet size in bytes */
    max_packet_size: number;
    /** Whether compression is enabled */
    compression_enabled: boolean;
    /** Whether encryption is enabled */
    encryption_enabled: boolean;
    /** Keep-alive interval in seconds */
    keep_alive_interval_secs: number;
    /** Connection timeout in seconds */
    timeout_secs: number;
}

/**
 * Request to update protocol settings
 */
export interface UpdateProtocolRequest {
    /** Update maximum packet size */
    max_packet_size?: number;
    /** Enable/disable compression */
    compression_enabled?: boolean;
    /** Update keep-alive interval */
    keep_alive_interval_secs?: number;
    /** Update connection timeout */
    timeout_secs?: number;
}

// ============================================================================
// Cluster Interfaces
// ============================================================================

/**
 * Overall cluster status
 */
export interface ClusterStatus {
    /** Unique cluster identifier */
    cluster_id: string;
    /** Cluster health status: healthy, degraded, unhealthy */
    status: string;
    /** Total number of nodes in the cluster */
    node_count: number;
    /** Number of healthy nodes */
    healthy_nodes: number;
    /** ID of the current leader node */
    leader_node_id?: string;
    /** Consensus algorithm in use (e.g., "raft") */
    consensus_algorithm: string;
    /** Replication factor */
    replication_factor: number;
}

/**
 * Detailed information about a cluster node
 */
export interface ClusterNode {
    /** Unique node identifier */
    node_id: string;
    /** Node IP address */
    address: string;
    /** Node port number */
    port: number;
    /** Node role: leader, follower, candidate */
    role: string;
    /** Node health status: healthy, degraded, unhealthy, offline */
    status: string;
    /** Software version running on the node */
    version: string;
    /** Node uptime in seconds */
    uptime_seconds: number;
    /** Unix timestamp of last heartbeat */
    last_heartbeat: number;
    /** Current CPU usage percentage */
    cpu_usage: number;
    /** Memory usage in megabytes */
    memory_usage_mb: number;
    /** Disk usage percentage */
    disk_usage_percent: number;
    /** Number of active connections to this node */
    connections: number;
}

/**
 * Request to add a new node to the cluster
 */
export interface AddClusterNodeRequest {
    /** Unique node identifier */
    node_id: string;
    /** Node IP address */
    address: string;
    /** Node port number */
    port: number;
    /** Optional node role (defaults to "follower") */
    role?: string;
}

// ============================================================================
// Load Balancer Interfaces
// ============================================================================

/**
 * Load balancer statistics and configuration
 */
export interface LoadBalancerStats {
    /** Load balancing algorithm: round_robin, least_connections, weighted */
    algorithm: string;
    /** Total requests processed */
    total_requests: number;
    /** Current requests per second rate */
    requests_per_second: number;
    /** Backend server pools */
    backend_pools: BackendPool[];
}

/**
 * Backend server pool information
 */
export interface BackendPool {
    /** Unique pool identifier */
    pool_id: string;
    /** List of backend servers in this pool */
    backends: Backend[];
    /** Currently active requests */
    active_requests: number;
    /** Total requests handled by this pool */
    total_requests: number;
}

/**
 * Individual backend server information
 */
export interface Backend {
    /** Unique backend identifier */
    backend_id: string;
    /** Backend server address (IP:port) */
    address: string;
    /** Backend weight for weighted load balancing */
    weight: number;
    /** Whether backend is active */
    active: boolean;
    /** Backend health status */
    health_status: string;
    /** Current active connections to backend */
    active_connections: number;
    /** Total requests sent to backend */
    total_requests: number;
    /** Total failed requests */
    failed_requests: number;
    /** Average response time in milliseconds */
    avg_response_time_ms: number;
}

/**
 * Load balancer configuration request
 */
export interface LoadBalancerConfig {
    /** Load balancing algorithm */
    algorithm: string;
    /** Health check interval in seconds */
    health_check_interval_secs: number;
    /** Maximum retry attempts */
    max_retries: number;
    /** Request timeout in seconds */
    timeout_secs: number;
}

/**
 * Circuit breaker status information
 */
export interface CircuitBreakerStatus {
    /** Circuit breaker name */
    name: string;
    /** Current state: closed, open, half_open */
    state: string;
    /** Total failure count */
    failure_count: number;
    /** Total success count */
    success_count: number;
    /** Unix timestamp of last failure */
    last_failure?: number;
    /** Unix timestamp of last state change */
    last_state_change: number;
    /** Failure threshold before opening */
    failure_threshold: number;
    /** Timeout in seconds before attempting recovery */
    timeout_secs: number;
}

// ============================================================================
// Connection Pool Interfaces
// ============================================================================

/**
 * Connection pool configuration
 */
export interface PoolConfig {
    /** Unique pool identifier */
    pool_id: string;
    /** Minimum number of connections to maintain */
    min_connections: number;
    /** Maximum number of allowed connections */
    max_connections: number;
    /** Connection acquisition timeout in seconds */
    connection_timeout_secs: number;
    /** Idle connection timeout in seconds */
    idle_timeout_secs: number;
    /** Maximum connection lifetime in seconds (optional) */
    max_lifetime_secs?: number;
}

/**
 * Connection pool statistics
 */
export interface PoolStatsResponse {
    /** Pool identifier */
    pool_id: string;
    /** Number of active connections */
    active_connections: number;
    /** Number of idle connections */
    idle_connections: number;
    /** Total connections (active + idle) */
    total_connections: number;
    /** Number of waiting requests */
    waiting_requests: number;
    /** Total connections acquired over lifetime */
    total_acquired: number;
    /** Total connections created */
    total_created: number;
    /** Total connections destroyed */
    total_destroyed: number;
}

/**
 * Individual connection information
 */
export interface ConnectionInfo {
    /** Unique connection identifier */
    connection_id: number;
    /** Pool this connection belongs to */
    pool_id: string;
    /** Associated session ID */
    session_id: number;
    /** Username associated with connection */
    username: string;
    /** Database name */
    database: string;
    /** Client address */
    client_address: string;
    /** Unix timestamp when connection was created */
    created_at: number;
    /** Unix timestamp of last activity */
    last_activity: number;
    /** Total queries executed on this connection */
    queries_executed: number;
    /** Connection state */
    state: string;
    /** Idle time in seconds */
    idle_time_secs: number;
}

/**
 * Session information
 */
export interface SessionInfo {
    /** Unique session identifier */
    session_id: number;
    /** Username */
    username: string;
    /** Database name */
    database: string;
    /** Client address */
    client_address?: string;
    /** Unix timestamp when session was created */
    created_at: number;
    /** Unix timestamp of last activity */
    last_activity: number;
    /** Current session state */
    state: string;
    /** Currently executing query (if any) */
    current_query?: string;
    /** Associated transaction ID (if in transaction) */
    transaction_id?: number;
}

/**
 * Pagination parameters for list requests
 */
export interface PaginationParams {
    /** Page number (1-indexed) */
    page?: number;
    /** Number of items per page */
    page_size?: number;
    /** Field to sort by */
    sort_by?: string;
    /** Sort order: asc, desc */
    sort_order?: string;
}

/**
 * Paginated response wrapper
 */
export interface PaginatedResponse<T> {
    /** Data items for current page */
    data: T[];
    /** Current page number */
    page: number;
    /** Items per page */
    page_size: number;
    /** Total number of pages */
    total_pages: number;
    /** Total number of items */
    total_count: number;
    /** Whether there is a next page */
    has_next: boolean;
    /** Whether there is a previous page */
    has_prev: boolean;
}

// ============================================================================
// Network & Pool API Client
// ============================================================================

/**
 * Client for Network & Connection Pool Management APIs
 */
export class NetworkPoolClient {
    private baseUrl: string;
    private headers: Record<string, string>;

    /**
     * Create a new NetworkPoolClient
     * @param baseUrl - Base URL of the RustyDB API (e.g., "http://localhost:8080")
     * @param apiKey - Optional API key for authentication
     */
    constructor(baseUrl: string, apiKey?: string) {
        this.baseUrl = baseUrl.replace(/\/$/, ''); // Remove trailing slash
        this.headers = {
            'Content-Type': 'application/json',
        };
        if (apiKey) {
            this.headers['Authorization'] = `Bearer ${apiKey}`;
        }
    }

    // ========================================================================
    // Network Status & Connections
    // ========================================================================

    /**
     * Get overall network status
     * GET /api/v1/network/status
     */
    async getNetworkStatus(): Promise<NetworkStatus> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/status`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get network status: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Get all active network connections
     * GET /api/v1/network/connections
     */
    async getNetworkConnections(): Promise<NetworkConnectionInfo[]> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/connections`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get network connections: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Get details of a specific network connection
     * GET /api/v1/network/connections/{id}
     */
    async getNetworkConnection(connectionId: string): Promise<NetworkConnectionInfo> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/connections/${connectionId}`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get network connection ${connectionId}: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Kill a specific network connection
     * DELETE /api/v1/network/connections/{id}
     */
    async killNetworkConnection(connectionId: string): Promise<void> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/connections/${connectionId}`, {
            method: 'DELETE',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to kill network connection ${connectionId}: ${response.statusText}`);
        }
    }

    // ========================================================================
    // Protocol Configuration
    // ========================================================================

    /**
     * Get current protocol configuration
     * GET /api/v1/network/protocols
     */
    async getProtocolConfig(): Promise<ProtocolConfig> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/protocols`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get protocol config: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Update protocol configuration
     * PUT /api/v1/network/protocols
     */
    async updateProtocolConfig(config: UpdateProtocolRequest): Promise<ProtocolConfig> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/protocols`, {
            method: 'PUT',
            headers: this.headers,
            body: JSON.stringify(config),
        });
        if (!response.ok) {
            throw new Error(`Failed to update protocol config: ${response.statusText}`);
        }
        return response.json();
    }

    // ========================================================================
    // Cluster Management
    // ========================================================================

    /**
     * Get overall cluster status
     * GET /api/v1/network/cluster/status
     */
    async getClusterStatus(): Promise<ClusterStatus> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/cluster/status`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get cluster status: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Get all cluster nodes
     * GET /api/v1/network/cluster/nodes
     */
    async getClusterNodes(): Promise<ClusterNode[]> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/cluster/nodes`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get cluster nodes: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Add a new node to the cluster
     * POST /api/v1/network/cluster/nodes
     */
    async addClusterNode(node: AddClusterNodeRequest): Promise<ClusterNode> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/cluster/nodes`, {
            method: 'POST',
            headers: this.headers,
            body: JSON.stringify(node),
        });
        if (!response.ok) {
            throw new Error(`Failed to add cluster node: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Remove a node from the cluster
     * DELETE /api/v1/network/cluster/nodes/{id}
     */
    async removeClusterNode(nodeId: string): Promise<void> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/cluster/nodes/${nodeId}`, {
            method: 'DELETE',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to remove cluster node ${nodeId}: ${response.statusText}`);
        }
    }

    // ========================================================================
    // Load Balancer
    // ========================================================================

    /**
     * Get load balancer statistics
     * GET /api/v1/network/loadbalancer
     */
    async getLoadBalancerStats(): Promise<LoadBalancerStats> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/loadbalancer`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get load balancer stats: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Configure load balancer settings
     * PUT /api/v1/network/loadbalancer/config
     */
    async configureLoadBalancer(config: LoadBalancerConfig): Promise<{ status: string; timestamp: number }> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/loadbalancer/config`, {
            method: 'PUT',
            headers: this.headers,
            body: JSON.stringify(config),
        });
        if (!response.ok) {
            throw new Error(`Failed to configure load balancer: ${response.statusText}`);
        }
        return response.json();
    }

    // ========================================================================
    // Circuit Breakers
    // ========================================================================

    /**
     * Get all circuit breaker statuses
     * GET /api/v1/network/circuit-breakers
     */
    async getCircuitBreakers(): Promise<CircuitBreakerStatus[]> {
        const response = await fetch(`${this.baseUrl}/api/v1/network/circuit-breakers`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get circuit breakers: ${response.statusText}`);
        }
        return response.json();
    }

    // ========================================================================
    // Connection Pools
    // ========================================================================

    /**
     * Get all connection pools
     * GET /api/v1/pools
     */
    async getPools(): Promise<PoolConfig[]> {
        const response = await fetch(`${this.baseUrl}/api/v1/pools`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get pools: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Get a specific connection pool configuration
     * GET /api/v1/pools/{id}
     */
    async getPool(poolId: string): Promise<PoolConfig> {
        const response = await fetch(`${this.baseUrl}/api/v1/pools/${poolId}`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get pool ${poolId}: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Update connection pool configuration
     * PUT /api/v1/pools/{id}
     */
    async updatePool(poolId: string, config: PoolConfig): Promise<void> {
        const response = await fetch(`${this.baseUrl}/api/v1/pools/${poolId}`, {
            method: 'PUT',
            headers: this.headers,
            body: JSON.stringify(config),
        });
        if (!response.ok) {
            throw new Error(`Failed to update pool ${poolId}: ${response.statusText}`);
        }
    }

    /**
     * Get connection pool statistics
     * GET /api/v1/pools/{id}/stats
     */
    async getPoolStats(poolId: string): Promise<PoolStatsResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/pools/${poolId}/stats`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get pool stats for ${poolId}: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Drain a connection pool (close idle connections)
     * POST /api/v1/pools/{id}/drain
     */
    async drainPool(poolId: string): Promise<void> {
        const response = await fetch(`${this.baseUrl}/api/v1/pools/${poolId}/drain`, {
            method: 'POST',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to drain pool ${poolId}: ${response.statusText}`);
        }
    }

    // ========================================================================
    // Connections
    // ========================================================================

    /**
     * Get all active connections with pagination
     * GET /api/v1/connections
     */
    async getConnections(params?: PaginationParams): Promise<PaginatedResponse<ConnectionInfo>> {
        const queryParams = new URLSearchParams();
        if (params?.page) {
queryParams.set('page', params.page.toString());
}
        if (params?.page_size) {
queryParams.set('page_size', params.page_size.toString());
}
        if (params?.sort_by) {
queryParams.set('sort_by', params.sort_by);
}
        if (params?.sort_order) {
queryParams.set('sort_order', params.sort_order);
}

        const url = `${this.baseUrl}/api/v1/connections${queryParams.toString() ? '?' + queryParams.toString() : ''}`;
        const response = await fetch(url, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get connections: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Get a specific connection by ID
     * GET /api/v1/connections/{id}
     */
    async getConnection(connectionId: number): Promise<ConnectionInfo> {
        const response = await fetch(`${this.baseUrl}/api/v1/connections/${connectionId}`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get connection ${connectionId}: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Kill a specific connection
     * DELETE /api/v1/connections/{id}
     */
    async killConnection(connectionId: number): Promise<void> {
        const response = await fetch(`${this.baseUrl}/api/v1/connections/${connectionId}`, {
            method: 'DELETE',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to kill connection ${connectionId}: ${response.statusText}`);
        }
    }

    // ========================================================================
    // Sessions
    // ========================================================================

    /**
     * Get all active sessions with pagination
     * GET /api/v1/sessions
     */
    async getSessions(params?: PaginationParams): Promise<PaginatedResponse<SessionInfo>> {
        const queryParams = new URLSearchParams();
        if (params?.page) {
queryParams.set('page', params.page.toString());
}
        if (params?.page_size) {
queryParams.set('page_size', params.page_size.toString());
}
        if (params?.sort_by) {
queryParams.set('sort_by', params.sort_by);
}
        if (params?.sort_order) {
queryParams.set('sort_order', params.sort_order);
}

        const url = `${this.baseUrl}/api/v1/sessions${queryParams.toString() ? '?' + queryParams.toString() : ''}`;
        const response = await fetch(url, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get sessions: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Get a specific session by ID
     * GET /api/v1/sessions/{id}
     */
    async getSession(sessionId: number): Promise<SessionInfo> {
        const response = await fetch(`${this.baseUrl}/api/v1/sessions/${sessionId}`, {
            method: 'GET',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to get session ${sessionId}: ${response.statusText}`);
        }
        return response.json();
    }

    /**
     * Terminate a specific session
     * DELETE /api/v1/sessions/{id}
     */
    async terminateSession(sessionId: number): Promise<void> {
        const response = await fetch(`${this.baseUrl}/api/v1/sessions/${sessionId}`, {
            method: 'DELETE',
            headers: this.headers,
        });
        if (!response.ok) {
            throw new Error(`Failed to terminate session ${sessionId}: ${response.statusText}`);
        }
    }
}

/**
 * Default export
 */
export default NetworkPoolClient;
