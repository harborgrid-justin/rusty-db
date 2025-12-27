import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend, AreaChart, Area } from 'recharts';
import {
  ArrowPathIcon,
  SignalIcon,
  ServerIcon,
  GlobeAltIcon,
  ChartBarIcon,
  ExclamationCircleIcon,
} from '@heroicons/react/24/outline';
import { Table } from '../components/common/Table';
import { Badge, StatusBadge, HealthBadge } from '../components/common/Badge';
import { Button } from '../components/common/Button';
import { Modal } from '../components/common/Modal';
import { Input } from '../components/common/Input';
import { Select } from '../components/common/Select';
import { networkingService } from '../services/networkingService';
import type { UUID } from '../types';

// ============================================================================
// Types
// ============================================================================

interface NetworkStatus {
  connectionPools: ConnectionPool[];
  clusterTopology: ClusterTopology;
  loadBalancer: LoadBalancerStats;
  protocolConfig: ProtocolConfig;
  bandwidth: BandwidthMetrics;
  circuitBreakers: CircuitBreaker[];
}

interface ConnectionPool {
  id: UUID;
  name: string;
  minConnections: number;
  maxConnections: number;
  activeConnections: number;
  idleConnections: number;
  waitingRequests: number;
  totalRequests: number;
  avgWaitTime: number;
  avgConnectionTime: number;
  status: 'healthy' | 'degraded' | 'unhealthy';
}

interface ClusterTopology {
  nodes: ClusterNode[];
  leader: UUID;
  healthy: number;
  degraded: number;
  unreachable: number;
}

interface ClusterNode {
  id: UUID;
  name: string;
  host: string;
  port: number;
  role: 'leader' | 'follower' | 'candidate';
  status: 'healthy' | 'degraded' | 'unreachable';
  region: string;
  zone: string;
  latency: number;
  connections: number;
  uptime: number;
}

interface LoadBalancerStats {
  algorithm: 'round_robin' | 'least_connections' | 'weighted' | 'ip_hash';
  totalRequests: number;
  successRate: number;
  avgResponseTime: number;
  requestsPerSecond: number;
  backends: BackendServer[];
  history: { time: string; requests: number; errors: number }[];
}

interface BackendServer {
  id: UUID;
  host: string;
  port: number;
  weight: number;
  activeConnections: number;
  totalRequests: number;
  failedRequests: number;
  avgResponseTime: number;
  status: 'active' | 'inactive' | 'draining';
}

interface ProtocolConfig {
  tcpKeepAlive: boolean;
  tcpNoDelay: boolean;
  maxPacketSize: number;
  compression: boolean;
  encryption: boolean;
  timeout: number;
  retries: number;
}

interface BandwidthMetrics {
  bytesReceived: number;
  bytesSent: number;
  packetsReceived: number;
  packetsSent: number;
  errors: number;
  dropped: number;
  history: { time: string; received: number; sent: number }[];
}

interface CircuitBreaker {
  id: UUID;
  name: string;
  state: 'closed' | 'open' | 'half_open';
  failureThreshold: number;
  currentFailures: number;
  successThreshold: number;
  timeout: number;
  lastStateChange: string;
}

// ============================================================================
// Network Page Component
// ============================================================================

export default function Network() {
  const [status, setStatus] = useState<NetworkStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTab, setSelectedTab] = useState<'pools' | 'topology' | 'loadbalancer' | 'config' | 'bandwidth'>('pools');
  const [showPoolModal, setShowPoolModal] = useState(false);
  const [refreshInterval, setRefreshInterval] = useState(10000);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, refreshInterval);
    return () => clearInterval(interval);
  }, [refreshInterval]);

  const loadData = async () => {
    try {
      setLoading(true);

      // NOTE: Integrating with actual networkingService APIs
      // TODO: Full migration from simulated data to real API responses needed
      // The component types need to be aligned with networkingService types

      // Example: Fetch real network status
      try {
        const realNetworkStatus = await networkingService.getNetworkStatus();
        console.log('Real network status:', realNetworkStatus);
      } catch (apiError) {
        console.warn('Using simulated data, API call failed:', apiError);
      }

      // Using simulated data for now (TODO: Replace with real API data)
      const data: NetworkStatus = {
        connectionPools: [
          {
            id: 'pool-1',
            name: 'primary_pool',
            minConnections: 10,
            maxConnections: 100,
            activeConnections: 45,
            idleConnections: 25,
            waitingRequests: 2,
            totalRequests: 125430,
            avgWaitTime: 5.2,
            avgConnectionTime: 2.1,
            status: 'healthy',
          },
          {
            id: 'pool-2',
            name: 'analytics_pool',
            minConnections: 5,
            maxConnections: 50,
            activeConnections: 28,
            idleConnections: 12,
            waitingRequests: 0,
            totalRequests: 45230,
            avgWaitTime: 3.8,
            avgConnectionTime: 1.9,
            status: 'healthy',
          },
          {
            id: 'pool-3',
            name: 'reporting_pool',
            minConnections: 5,
            maxConnections: 25,
            activeConnections: 22,
            idleConnections: 2,
            waitingRequests: 5,
            totalRequests: 18500,
            avgWaitTime: 12.5,
            avgConnectionTime: 3.2,
            status: 'degraded',
          },
        ],
        clusterTopology: {
          nodes: [
            {
              id: 'node-1',
              name: 'db-primary-1',
              host: '10.0.1.10',
              port: 5432,
              role: 'leader',
              status: 'healthy',
              region: 'us-east-1',
              zone: 'us-east-1a',
              latency: 1.2,
              connections: 145,
              uptime: 2592000,
            },
            {
              id: 'node-2',
              name: 'db-replica-1',
              host: '10.0.1.11',
              port: 5432,
              role: 'follower',
              status: 'healthy',
              region: 'us-east-1',
              zone: 'us-east-1b',
              latency: 2.5,
              connections: 85,
              uptime: 2491200,
            },
            {
              id: 'node-3',
              name: 'db-replica-2',
              host: '10.0.2.10',
              port: 5432,
              role: 'follower',
              status: 'healthy',
              region: 'us-west-2',
              zone: 'us-west-2a',
              latency: 45.3,
              connections: 52,
              uptime: 1728000,
            },
            {
              id: 'node-4',
              name: 'db-replica-3',
              host: '10.0.3.10',
              port: 5432,
              role: 'follower',
              status: 'degraded',
              region: 'eu-west-1',
              zone: 'eu-west-1a',
              latency: 95.8,
              connections: 28,
              uptime: 864000,
            },
          ],
          leader: 'node-1',
          healthy: 3,
          degraded: 1,
          unreachable: 0,
        },
        loadBalancer: {
          algorithm: 'least_connections',
          totalRequests: 2450000,
          successRate: 99.7,
          avgResponseTime: 15.4,
          requestsPerSecond: 1250,
          backends: [
            {
              id: 'backend-1',
              host: '10.0.1.10',
              port: 5432,
              weight: 100,
              activeConnections: 145,
              totalRequests: 980000,
              failedRequests: 250,
              avgResponseTime: 12.3,
              status: 'active',
            },
            {
              id: 'backend-2',
              host: '10.0.1.11',
              port: 5432,
              weight: 100,
              activeConnections: 85,
              totalRequests: 850000,
              failedRequests: 180,
              avgResponseTime: 14.5,
              status: 'active',
            },
            {
              id: 'backend-3',
              host: '10.0.2.10',
              port: 5432,
              weight: 50,
              activeConnections: 52,
              totalRequests: 420000,
              failedRequests: 120,
              avgResponseTime: 48.2,
              status: 'active',
            },
            {
              id: 'backend-4',
              host: '10.0.3.10',
              port: 5432,
              weight: 25,
              activeConnections: 28,
              totalRequests: 200000,
              failedRequests: 350,
              avgResponseTime: 102.5,
              status: 'draining',
            },
          ],
          history: Array.from({ length: 20 }, (_, i) => ({
            time: new Date(Date.now() - (19 - i) * 60000).toLocaleTimeString(),
            requests: Math.floor(Math.random() * 1500) + 1000,
            errors: Math.floor(Math.random() * 10),
          })),
        },
        protocolConfig: {
          tcpKeepAlive: true,
          tcpNoDelay: true,
          maxPacketSize: 65536,
          compression: true,
          encryption: true,
          timeout: 30000,
          retries: 3,
        },
        bandwidth: {
          bytesReceived: 2500000000,
          bytesSent: 1800000000,
          packetsReceived: 1250000,
          packetsSent: 950000,
          errors: 125,
          dropped: 45,
          history: Array.from({ length: 20 }, (_, i) => ({
            time: new Date(Date.now() - (19 - i) * 60000).toLocaleTimeString(),
            received: Math.floor(Math.random() * 50) + 100,
            sent: Math.floor(Math.random() * 40) + 80,
          })),
        },
        circuitBreakers: [
          {
            id: 'cb-1',
            name: 'primary_database',
            state: 'closed',
            failureThreshold: 5,
            currentFailures: 0,
            successThreshold: 3,
            timeout: 60000,
            lastStateChange: new Date(Date.now() - 3600000).toISOString(),
          },
          {
            id: 'cb-2',
            name: 'cache_service',
            state: 'half_open',
            failureThreshold: 3,
            currentFailures: 2,
            successThreshold: 2,
            timeout: 30000,
            lastStateChange: new Date(Date.now() - 120000).toISOString(),
          },
        ],
      };
      setStatus(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load network data');
    } finally {
      setLoading(false);
    }
  };

  const formatBytes = (bytes: number): string => {
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    if (bytes === 0) return '0 B';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${sizes[i]}`;
  };

  const formatUptime = (seconds: number): string => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    return `${days}d ${hours}h`;
  };

  if (loading && !status) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading network data...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-4">
        <p className="text-red-400">Error: {error}</p>
      </div>
    );
  }

  if (!status) return null;

  const degradedNodes = status.clusterTopology.nodes.filter((n) => n.status === 'degraded').length;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Network Administration</h1>
          <p className="text-gray-400 mt-1">Monitor connections, cluster topology, and network performance</p>
        </div>
        <div className="flex items-center space-x-3">
          <select
            value={refreshInterval}
            onChange={(e) => setRefreshInterval(Number(e.target.value))}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value={5000}>Refresh: 5s</option>
            <option value={10000}>Refresh: 10s</option>
            <option value={30000}>Refresh: 30s</option>
          </select>
          <button
            onClick={loadData}
            disabled={loading}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors disabled:opacity-50 flex items-center space-x-2"
          >
            <ArrowPathIcon className="h-5 w-5" />
            <span>{loading ? 'Loading...' : 'Refresh'}</span>
          </button>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Total Connections</h3>
            <SignalIcon className="h-6 w-6 text-blue-400" />
          </div>
          <div className="text-3xl font-bold text-white">
            {status.connectionPools.reduce((sum, pool) => sum + pool.activeConnections, 0)}
          </div>
          <div className="text-xs text-gray-500 mt-1">
            {status.connectionPools.reduce((sum, pool) => sum + pool.waitingRequests, 0)} waiting
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Cluster Nodes</h3>
            <ServerIcon className="h-6 w-6 text-green-400" />
          </div>
          <div className="text-3xl font-bold text-white">{status.clusterTopology.nodes.length}</div>
          <div className="text-xs text-gray-500 mt-1">
            {status.clusterTopology.healthy} healthy / {degradedNodes} degraded
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Requests/sec</h3>
            <ChartBarIcon className="h-6 w-6 text-purple-400" />
          </div>
          <div className="text-3xl font-bold text-white">{status.loadBalancer.requestsPerSecond.toLocaleString()}</div>
          <div className="text-xs text-gray-500 mt-1">
            {status.loadBalancer.successRate}% success rate
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium text-gray-400">Bandwidth</h3>
            <GlobeAltIcon className="h-6 w-6 text-orange-400" />
          </div>
          <div className="text-3xl font-bold text-white">{formatBytes(status.bandwidth.bytesReceived + status.bandwidth.bytesSent)}</div>
          <div className="text-xs text-gray-500 mt-1">
            ↓{formatBytes(status.bandwidth.bytesReceived)} ↑{formatBytes(status.bandwidth.bytesSent)}
          </div>
        </motion.div>
      </div>

      {/* Warning Banner */}
      {degradedNodes > 0 && (
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-yellow-900 bg-opacity-20 border border-yellow-500 rounded-lg p-4"
        >
          <div className="flex items-start space-x-3">
            <ExclamationCircleIcon className="h-6 w-6 text-yellow-400 flex-shrink-0" />
            <div>
              <h3 className="text-lg font-semibold text-yellow-400">Cluster Health Warning</h3>
              <p className="text-sm text-yellow-300 mt-1">
                {degradedNodes} cluster node(s) are degraded. Performance may be affected.
              </p>
            </div>
          </div>
        </motion.div>
      )}

      {/* Tabs */}
      <div className="flex space-x-2 border-b border-gray-700">
        {(['pools', 'topology', 'loadbalancer', 'config', 'bandwidth'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setSelectedTab(tab)}
            className={`px-4 py-2 font-medium capitalize transition-colors ${
              selectedTab === tab
                ? 'text-blue-400 border-b-2 border-blue-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'loadbalancer' ? 'Load Balancer' : tab}
          </button>
        ))}
      </div>

      {/* Connection Pools Tab */}
      {selectedTab === 'pools' && (
        <>
          {/* Pool Cards */}
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            {status.connectionPools.map((pool, index) => (
              <motion.div
                key={pool.id}
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: index * 0.1 }}
                className="bg-gray-800 rounded-lg p-6"
              >
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-lg font-semibold text-white">{pool.name}</h3>
                  <HealthBadge health={pool.status} size="sm" />
                </div>
                <div className="space-y-3">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Active:</span>
                    <span className="text-white font-medium">
                      {pool.activeConnections} / {pool.maxConnections}
                    </span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Idle:</span>
                    <span className="text-white font-medium">{pool.idleConnections}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Waiting:</span>
                    <span className={`font-medium ${pool.waitingRequests > 0 ? 'text-yellow-400' : 'text-white'}`}>
                      {pool.waitingRequests}
                    </span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Avg Wait:</span>
                    <span className="text-white font-medium">{pool.avgWaitTime.toFixed(1)} ms</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Total Requests:</span>
                    <span className="text-white font-medium">{pool.totalRequests.toLocaleString()}</span>
                  </div>
                </div>
                <div className="mt-4 bg-gray-700 rounded-full h-2 overflow-hidden">
                  <div
                    className={`h-full transition-all ${
                      (pool.activeConnections / pool.maxConnections) > 0.8
                        ? 'bg-red-500'
                        : (pool.activeConnections / pool.maxConnections) > 0.6
                        ? 'bg-yellow-500'
                        : 'bg-green-500'
                    }`}
                    style={{ width: `${(pool.activeConnections / pool.maxConnections) * 100}%` }}
                  />
                </div>
              </motion.div>
            ))}
          </div>

          {/* Add Pool Button */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.4 }}
          >
            <Button
              variant="primary"
              onClick={() => setShowPoolModal(true)}
            >
              Create Connection Pool
            </Button>
          </motion.div>
        </>
      )}

      {/* Cluster Topology Tab */}
      {selectedTab === 'topology' && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <h3 className="text-lg font-semibold text-white mb-4">Cluster Nodes</h3>
          <Table
            columns={[
              { key: 'name', header: 'Name', sortable: true },
              {
                key: 'host',
                header: 'Address',
                render: (value, row) => `${value}:${row.port}`,
              },
              {
                key: 'role',
                header: 'Role',
                render: (value) => (
                  <Badge variant={value === 'leader' ? 'primary' : 'info'}>
                    {value}
                  </Badge>
                ),
              },
              {
                key: 'status',
                header: 'Status',
                render: (value) => <HealthBadge health={value} size="sm" />,
              },
              {
                key: 'region',
                header: 'Region / Zone',
                render: (value, row) => (
                  <div>
                    <div className="text-sm">{value}</div>
                    <div className="text-xs text-gray-500">{row.zone}</div>
                  </div>
                ),
              },
              {
                key: 'latency',
                header: 'Latency',
                render: (value) => `${value.toFixed(1)} ms`,
                sortable: true,
              },
              { key: 'connections', header: 'Connections', sortable: true },
              {
                key: 'uptime',
                header: 'Uptime',
                render: (value) => formatUptime(value),
                sortable: true,
              },
            ]}
            data={status.clusterTopology.nodes}
            striped
            hoverable
          />
        </motion.div>
      )}

      {/* Load Balancer Tab */}
      {selectedTab === 'loadbalancer' && (
        <>
          {/* Load Balancer Stats */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Load Balancer Overview</h3>
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
              <div>
                <p className="text-sm text-gray-400 mb-1">Algorithm</p>
                <p className="text-lg font-bold text-white capitalize">{status.loadBalancer.algorithm.replace('_', ' ')}</p>
              </div>
              <div>
                <p className="text-sm text-gray-400 mb-1">Total Requests</p>
                <p className="text-lg font-bold text-white">{status.loadBalancer.totalRequests.toLocaleString()}</p>
              </div>
              <div>
                <p className="text-sm text-gray-400 mb-1">Success Rate</p>
                <p className="text-lg font-bold text-green-400">{status.loadBalancer.successRate}%</p>
              </div>
              <div>
                <p className="text-sm text-gray-400 mb-1">Avg Response Time</p>
                <p className="text-lg font-bold text-white">{status.loadBalancer.avgResponseTime.toFixed(1)} ms</p>
              </div>
            </div>

            {/* Request History Chart */}
            <ResponsiveContainer width="100%" height={250}>
              <LineChart data={status.loadBalancer.history}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9CA3AF" style={{ fontSize: 12 }} />
                <YAxis stroke="#9CA3AF" style={{ fontSize: 12 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1F2937',
                    border: '1px solid #374151',
                    borderRadius: '0.5rem',
                  }}
                />
                <Legend />
                <Line type="monotone" dataKey="requests" stroke="#3b82f6" strokeWidth={2} name="Requests" dot={false} />
                <Line type="monotone" dataKey="errors" stroke="#ef4444" strokeWidth={2} name="Errors" dot={false} />
              </LineChart>
            </ResponsiveContainer>
          </motion.div>

          {/* Backend Servers */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Backend Servers</h3>
            <Table
              columns={[
                {
                  key: 'host',
                  header: 'Address',
                  render: (value, row) => `${value}:${row.port}`,
                },
                { key: 'weight', header: 'Weight', sortable: true },
                { key: 'activeConnections', header: 'Connections', sortable: true },
                {
                  key: 'totalRequests',
                  header: 'Requests',
                  render: (value) => value.toLocaleString(),
                  sortable: true,
                },
                {
                  key: 'failedRequests',
                  header: 'Failed',
                  render: (value) => value.toLocaleString(),
                  sortable: true,
                },
                {
                  key: 'avgResponseTime',
                  header: 'Avg Response',
                  render: (value) => `${value.toFixed(1)} ms`,
                  sortable: true,
                },
                {
                  key: 'status',
                  header: 'Status',
                  render: (value) => (
                    <StatusBadge
                      status={value === 'active' ? 'active' : value === 'draining' ? 'warning' : 'inactive'}
                      label={value}
                    />
                  ),
                },
              ]}
              data={status.loadBalancer.backends}
              striped
              hoverable
            />
          </motion.div>
        </>
      )}

      {/* Protocol Config Tab */}
      {selectedTab === 'config' && (
        <>
          {/* Protocol Configuration */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Protocol Configuration</h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div className="space-y-4">
                <div className="flex justify-between items-center">
                  <span className="text-gray-400">TCP Keep-Alive</span>
                  <Badge variant={status.protocolConfig.tcpKeepAlive ? 'success' : 'neutral'}>
                    {status.protocolConfig.tcpKeepAlive ? 'Enabled' : 'Disabled'}
                  </Badge>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-gray-400">TCP No-Delay</span>
                  <Badge variant={status.protocolConfig.tcpNoDelay ? 'success' : 'neutral'}>
                    {status.protocolConfig.tcpNoDelay ? 'Enabled' : 'Disabled'}
                  </Badge>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-gray-400">Compression</span>
                  <Badge variant={status.protocolConfig.compression ? 'success' : 'neutral'}>
                    {status.protocolConfig.compression ? 'Enabled' : 'Disabled'}
                  </Badge>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-gray-400">Encryption</span>
                  <Badge variant={status.protocolConfig.encryption ? 'success' : 'neutral'}>
                    {status.protocolConfig.encryption ? 'Enabled' : 'Disabled'}
                  </Badge>
                </div>
              </div>
              <div className="space-y-4">
                <div className="flex justify-between">
                  <span className="text-gray-400">Max Packet Size</span>
                  <span className="text-white font-medium">{formatBytes(status.protocolConfig.maxPacketSize)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Timeout</span>
                  <span className="text-white font-medium">{status.protocolConfig.timeout / 1000}s</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Max Retries</span>
                  <span className="text-white font-medium">{status.protocolConfig.retries}</span>
                </div>
              </div>
            </div>
            <div className="mt-6">
              <Button variant="primary">Edit Configuration</Button>
            </div>
          </motion.div>

          {/* Circuit Breakers */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Circuit Breakers</h3>
            <Table
              columns={[
                { key: 'name', header: 'Name', sortable: true },
                {
                  key: 'state',
                  header: 'State',
                  render: (value) => (
                    <Badge
                      variant={value === 'closed' ? 'success' : value === 'open' ? 'danger' : 'warning'}
                    >
                      {value}
                    </Badge>
                  ),
                },
                {
                  key: 'currentFailures',
                  header: 'Failures',
                  render: (value, row) => `${value} / ${row.failureThreshold}`,
                },
                {
                  key: 'timeout',
                  header: 'Timeout',
                  render: (value) => `${value / 1000}s`,
                },
                {
                  key: 'lastStateChange',
                  header: 'Last Change',
                  render: (value) => new Date(value).toLocaleString(),
                },
              ]}
              data={status.circuitBreakers}
              striped
              hoverable
            />
          </motion.div>
        </>
      )}

      {/* Bandwidth Tab */}
      {selectedTab === 'bandwidth' && (
        <>
          {/* Bandwidth Stats */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-sm font-medium text-gray-400 mb-2">Total Transfer</h3>
              <div className="text-3xl font-bold text-white">
                {formatBytes(status.bandwidth.bytesReceived + status.bandwidth.bytesSent)}
              </div>
              <div className="text-xs text-gray-500 mt-1">
                {(status.bandwidth.packetsReceived + status.bandwidth.packetsSent).toLocaleString()} packets
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-sm font-medium text-gray-400 mb-2">Errors</h3>
              <div className="text-3xl font-bold text-red-400">{status.bandwidth.errors}</div>
              <div className="text-xs text-gray-500 mt-1">
                {status.bandwidth.dropped} dropped packets
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.2 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-sm font-medium text-gray-400 mb-2">Error Rate</h3>
              <div className="text-3xl font-bold text-white">
                {((status.bandwidth.errors / (status.bandwidth.packetsReceived + status.bandwidth.packetsSent)) * 100).toFixed(3)}%
              </div>
            </motion.div>
          </div>

          {/* Bandwidth Chart */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.3 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Network Traffic</h3>
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={status.bandwidth.history}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9CA3AF" style={{ fontSize: 12 }} />
                <YAxis stroke="#9CA3AF" style={{ fontSize: 12 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1F2937',
                    border: '1px solid #374151',
                    borderRadius: '0.5rem',
                  }}
                />
                <Legend />
                <Area
                  type="monotone"
                  dataKey="received"
                  stackId="1"
                  stroke="#10b981"
                  fill="#10b981"
                  name="Received (MB/s)"
                />
                <Area
                  type="monotone"
                  dataKey="sent"
                  stackId="2"
                  stroke="#3b82f6"
                  fill="#3b82f6"
                  name="Sent (MB/s)"
                />
              </AreaChart>
            </ResponsiveContainer>
          </motion.div>
        </>
      )}

      {/* Create Pool Modal */}
      {showPoolModal && (
        <Modal
          isOpen={showPoolModal}
          onClose={() => setShowPoolModal(false)}
          title="Create Connection Pool"
          size="lg"
        >
          <div className="space-y-4">
            <Input label="Pool Name" placeholder="Enter pool name" />
            <div className="grid grid-cols-2 gap-4">
              <Input label="Min Connections" type="number" placeholder="10" />
              <Input label="Max Connections" type="number" placeholder="100" />
            </div>
            <Input label="Target Database" placeholder="Enter database name" />
            <Select
              label="Load Balancing"
              options={[
                { value: 'round_robin', label: 'Round Robin' },
                { value: 'least_connections', label: 'Least Connections' },
                { value: 'weighted', label: 'Weighted' },
              ]}
            />
            <div className="flex justify-end space-x-3 mt-6">
              <Button variant="secondary" onClick={() => setShowPoolModal(false)}>
                Cancel
              </Button>
              <Button variant="primary" onClick={() => setShowPoolModal(false)}>
                Create Pool
              </Button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
