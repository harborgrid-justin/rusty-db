/**
 * RAC (Real Application Clusters) Management Page
 * Enterprise clustering, cache fusion, and GRD management
 */

import React, { useState, useEffect, useCallback } from 'react';
import {
  ServerIcon,
  CpuChipIcon,
  CircleStackIcon,
  ArrowPathIcon,
  ChartBarIcon,
  BoltIcon,
  SignalIcon,
} from '@heroicons/react/24/outline';
import racService, { type ClusterStatus } from '../services/racService';
import { toast } from 'react-hot-toast';

export default function RacManagement() {
  const [clusterStatus, setClusterStatus] = useState<ClusterStatus | null>(null);
  const [cacheFusionStats, setCacheFusionStats] = useState<unknown>(null);
  const [grdResources, setGrdResources] = useState<unknown[]>([]);
  const [interconnectStats, setInterconnectStats] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'overview' | 'cache-fusion' | 'grd' | 'interconnect'>('overview');

  const loadData = useCallback(async () => {
    try {
      if (activeTab === 'overview') {
        const statusRes = await racService.getClusterStatus();
        if (statusRes.success) setClusterStatus(statusRes.data);
      } else if (activeTab === 'cache-fusion') {
        const statsRes = await racService.getCacheFusionStats();
        if (statsRes.success) setCacheFusionStats(statsRes.data);
      } else if (activeTab === 'grd') {
        const resourcesRes = await racService.getGrdResources();
        if (resourcesRes.success) setGrdResources(resourcesRes.data);
      } else if (activeTab === 'interconnect') {
        const interconnectRes = await racService.getInterconnectStats();
        if (interconnectRes.success) setInterconnectStats(interconnectRes.data);
      }
    } catch (error) {
      console.error('Failed to load RAC data:', error);
    } finally {
      setLoading(false);
    }
  }, [activeTab]);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, 5000);
    return () => clearInterval(interval);
  }, [loadData]);

  const handleRebalance = async () => {
    try {
      const res = await racService.triggerRebalance();
      if (res.success) {
        toast.success(`Rebalance initiated: ${res.data.job_id}`);
        loadData();
      }
    } catch (error) {
      const err = error as Error;
      toast.error(`Failed to trigger rebalance: ${err.message}`);
    }
  };

  const handleFlushCache = async (nodeId?: string) => {
    try {
      const res = await racService.flushCacheFusion({ node_id: nodeId, force: true });
      if (res.success) {
        toast.success('Cache fusion flushed successfully');
        loadData();
      }
    } catch (error) {
      const err = error as Error;
      toast.error(`Failed to flush cache: ${err.message}`);
    }
  };

  const handleRemaster = async () => {
    try {
      const res = await racService.triggerRemaster({ strategy: 'graceful' });
      if (res.success) {
        toast.success(`Remastering initiated: ${res.data.job_id}`);
        loadData();
      }
    } catch (error) {
      const err = error as Error;
      toast.error(`Failed to trigger remastering: ${err.message}`);
    }
  };

  const getNodeStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'text-green-600 bg-green-100';
      case 'inactive': return 'text-gray-600 bg-gray-100';
      case 'failed': return 'text-red-600 bg-red-100';
      case 'joining': return 'text-yellow-600 bg-yellow-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  };

  if (loading && !clusterStatus) {
    return (
      <div className="flex items-center justify-center h-96">
        <ArrowPathIcon className="w-8 h-8 animate-spin text-blue-600" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">RAC Cluster Management</h1>
          <p className="mt-1 text-sm text-gray-500">
            Real Application Clusters - High availability and scalability
          </p>
        </div>
        <button
          onClick={handleRebalance}
          className="flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          <ArrowPathIcon className="w-5 h-5 mr-2" />
          Rebalance Cluster
        </button>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="-mb-px flex space-x-8">
          {[
            { id: 'overview', name: 'Cluster Overview', icon: ServerIcon },
            { id: 'cache-fusion', name: 'Cache Fusion', icon: BoltIcon },
            { id: 'grd', name: 'GRD Resources', icon: CircleStackIcon },
            { id: 'interconnect', name: 'Interconnect', icon: SignalIcon },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as 'overview' | 'cache-fusion' | 'grd' | 'interconnect')}
              className={`
                flex items-center py-4 px-1 border-b-2 font-medium text-sm
                ${activeTab === tab.id
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }
              `}
            >
              <tab.icon className="w-5 h-5 mr-2" />
              {tab.name}
            </button>
          ))}
        </nav>
      </div>

      {/* Content */}
      {activeTab === 'overview' && clusterStatus && (
        <div className="space-y-6">
          {/* Cluster Stats */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
            <div className="bg-white p-6 rounded-lg shadow">
              <div className="flex items-center">
                <ServerIcon className="w-8 h-8 text-blue-600" />
                <div className="ml-4">
                  <p className="text-sm text-gray-500">Total Nodes</p>
                  <p className="text-2xl font-bold">{clusterStatus.total_nodes}</p>
                </div>
              </div>
            </div>
            <div className="bg-white p-6 rounded-lg shadow">
              <div className="flex items-center">
                <div className="w-8 h-8 rounded-full bg-green-100 flex items-center justify-center">
                  <div className="w-4 h-4 rounded-full bg-green-600"></div>
                </div>
                <div className="ml-4">
                  <p className="text-sm text-gray-500">Active Nodes</p>
                  <p className="text-2xl font-bold text-green-600">{clusterStatus.active_nodes}</p>
                </div>
              </div>
            </div>
            <div className="bg-white p-6 rounded-lg shadow">
              <div className="flex items-center">
                <BoltIcon className="w-8 h-8 text-purple-600" />
                <div className="ml-4">
                  <p className="text-sm text-gray-500">Cache Fusion</p>
                  <p className="text-2xl font-bold">
                    {clusterStatus.cache_fusion_enabled ? 'Enabled' : 'Disabled'}
                  </p>
                </div>
              </div>
            </div>
            <div className="bg-white p-6 rounded-lg shadow">
              <div className="flex items-center">
                <CircleStackIcon className="w-8 h-8 text-indigo-600" />
                <div className="ml-4">
                  <p className="text-sm text-gray-500">GRD</p>
                  <p className="text-2xl font-bold">
                    {clusterStatus.grd_enabled ? 'Enabled' : 'Disabled'}
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Nodes Table */}
          <div className="bg-white rounded-lg shadow overflow-hidden">
            <div className="px-6 py-4 border-b border-gray-200">
              <h3 className="text-lg font-medium">Cluster Nodes</h3>
            </div>
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Node</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Role</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">CPU</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Memory</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Connections</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Last Heartbeat</th>
                  </tr>
                </thead>
                <tbody className="bg-white divide-y divide-gray-200">
                  {clusterStatus.nodes.map((node) => (
                    <tr key={node.id} className="hover:bg-gray-50">
                      <td className="px-6 py-4 whitespace-nowrap">
                        <div className="flex items-center">
                          <ServerIcon className="w-5 h-5 text-gray-400 mr-2" />
                          <div>
                            <div className="text-sm font-medium text-gray-900">{node.name}</div>
                            <div className="text-sm text-gray-500">{node.host}:{node.port}</div>
                          </div>
                        </div>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <span className={`px-2 py-1 text-xs font-medium rounded-full ${getNodeStatusColor(node.status)}`}>
                          {node.status}
                        </span>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 capitalize">
                        {node.role}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <div className="flex items-center">
                          <div className="w-16 bg-gray-200 rounded-full h-2 mr-2">
                            <div
                              className="bg-blue-600 h-2 rounded-full"
                              style={{ width: `${node.cpu_usage}%` }}
                            />
                          </div>
                          <span className="text-sm text-gray-900">{node.cpu_usage.toFixed(1)}%</span>
                        </div>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <div className="flex items-center">
                          <div className="w-16 bg-gray-200 rounded-full h-2 mr-2">
                            <div
                              className="bg-green-600 h-2 rounded-full"
                              style={{ width: `${(node.memory_used / node.memory_total) * 100}%` }}
                            />
                          </div>
                          <span className="text-sm text-gray-900">
                            {((node.memory_used / node.memory_total) * 100).toFixed(1)}%
                          </span>
                        </div>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                        {node.connections}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                        {new Date(node.last_heartbeat).toLocaleTimeString()}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {activeTab === 'cache-fusion' && cacheFusionStats && (
        <div className="space-y-6">
          {/* Cache Fusion Stats */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="bg-white p-6 rounded-lg shadow">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-500">Cache Hit Rate</p>
                  <p className="text-3xl font-bold text-green-600">
                    {(cacheFusionStats.cache_hit_rate * 100).toFixed(1)}%
                  </p>
                </div>
                <ChartBarIcon className="w-12 h-12 text-green-600" />
              </div>
            </div>
            <div className="bg-white p-6 rounded-lg shadow">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-500">Fusion Rate</p>
                  <p className="text-3xl font-bold text-blue-600">
                    {(cacheFusionStats.fusion_rate * 100).toFixed(1)}%
                  </p>
                </div>
                <BoltIcon className="w-12 h-12 text-blue-600" />
              </div>
            </div>
            <div className="bg-white p-6 rounded-lg shadow">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-500">Avg Transfer Time</p>
                  <p className="text-3xl font-bold text-purple-600">
                    {cacheFusionStats.avg_transfer_time_ms.toFixed(2)} ms
                  </p>
                </div>
                <CpuChipIcon className="w-12 h-12 text-purple-600" />
              </div>
            </div>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-medium">Cache Fusion Operations</h3>
              <button
                onClick={() => handleFlushCache()}
                className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
              >
                Flush Cache
              </button>
            </div>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="border rounded-lg p-4">
                <p className="text-sm text-gray-500">Total Transfers</p>
                <p className="text-2xl font-bold">{cacheFusionStats.total_transfers.toLocaleString()}</p>
              </div>
              <div className="border rounded-lg p-4">
                <p className="text-sm text-gray-500">Local Reads</p>
                <p className="text-2xl font-bold text-green-600">{cacheFusionStats.local_reads.toLocaleString()}</p>
              </div>
              <div className="border rounded-lg p-4">
                <p className="text-sm text-gray-500">Remote Reads</p>
                <p className="text-2xl font-bold text-orange-600">{cacheFusionStats.remote_reads.toLocaleString()}</p>
              </div>
              <div className="border rounded-lg p-4">
                <p className="text-sm text-gray-500">Block Pings</p>
                <p className="text-2xl font-bold text-blue-600">{cacheFusionStats.block_pings.toLocaleString()}</p>
              </div>
            </div>
          </div>
        </div>
      )}

      {activeTab === 'grd' && (
        <div className="space-y-6">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-medium">Global Resource Directory</h3>
            <button
              onClick={handleRemaster}
              className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700"
            >
              Trigger Remastering
            </button>
          </div>
          <div className="bg-white rounded-lg shadow overflow-hidden">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Resource</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Type</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Master Node</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Lock Count</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Avg Wait (ms)</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {grdResources.map((resource) => (
                  <tr key={resource.resource_id} className="hover:bg-gray-50">
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                      {resource.resource_name}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {resource.resource_type}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {resource.master_node}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {resource.lock_count}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {resource.average_wait_time_ms.toFixed(2)}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className="px-2 py-1 text-xs font-medium rounded-full bg-green-100 text-green-600">
                        {resource.status}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {activeTab === 'interconnect' && (
        <div className="space-y-6">
          <h3 className="text-lg font-medium">Interconnect Statistics</h3>
          <div className="bg-white rounded-lg shadow overflow-hidden">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">From → To</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Packets</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Bytes</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Latency (Avg)</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Latency (P99)</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Bandwidth</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Errors</th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {interconnectStats.map((stat, idx) => (
                  <tr key={idx} className="hover:bg-gray-50">
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                      {stat.node_from} → {stat.node_to}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {stat.packets_sent.toLocaleString()}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {(stat.bytes_sent / 1024 / 1024).toFixed(2)} MB
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {stat.latency_avg_ms.toFixed(2)} ms
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {stat.latency_p99_ms.toFixed(2)} ms
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {stat.bandwidth_mbps.toFixed(2)} Mbps
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`text-sm font-medium ${stat.errors > 0 ? 'text-red-600' : 'text-green-600'}`}>
                        {stat.errors}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
