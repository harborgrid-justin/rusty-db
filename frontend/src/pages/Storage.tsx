import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, Legend, BarChart, Bar, XAxis, YAxis, CartesianGrid, LineChart, Line } from 'recharts';
import {
  CircleStackIcon,
  ServerIcon,
  ChartPieIcon,
  CpuChipIcon,
  ArrowPathIcon,
  PlusIcon,
  TrashIcon,
  PencilIcon,
} from '@heroicons/react/24/outline';
import { Card, CardHeader } from '../components/common/Card';
import { Table } from '../components/common/Table';
import { Badge, StatusBadge } from '../components/common/Badge';
import { Button } from '../components/common/Button';
import { Modal } from '../components/common/Modal';
import { Input } from '../components/common/Input';
import { Select } from '../components/common/Select';
import type { UUID } from '../types';

// ============================================================================
// Types
// ============================================================================

interface StorageStatus {
  diskUsage: {
    total: number;
    used: number;
    free: number;
    databases: { name: string; size: number; color: string }[];
  };
  bufferPool: {
    totalSize: number;
    usedSize: number;
    dirtyPages: number;
    hitRatio: number;
    evictions: number;
  };
  ioMetrics: {
    readOps: number;
    writeOps: number;
    readBytes: number;
    writeBytes: number;
    avgReadLatency: number;
    avgWriteLatency: number;
  };
  tablespaces: Tablespace[];
  partitions: Partition[];
}

interface Tablespace {
  id: UUID;
  name: string;
  location: string;
  size: number;
  used: number;
  status: 'online' | 'offline' | 'readonly';
  tableCount: number;
}

interface Partition {
  id: UUID;
  tableName: string;
  partitionName: string;
  strategy: 'range' | 'hash' | 'list';
  size: number;
  rowCount: number;
  lastAccessed: string;
}

// ============================================================================
// Storage Page Component
// ============================================================================

export default function Storage() {
  const [status, setStatus] = useState<StorageStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTab, setSelectedTab] = useState<'overview' | 'tablespaces' | 'partitions'>('overview');
  const [showPartitionModal, setShowPartitionModal] = useState(false);
  const [refreshInterval, setRefreshInterval] = useState(30000);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, refreshInterval);
    return () => clearInterval(interval);
  }, [refreshInterval]);

  const loadData = async () => {
    try {
      setLoading(true);
      // Simulated API call - replace with actual service call
      // const data = await storageService.getStorageStatus();
      const data: StorageStatus = {
        diskUsage: {
          total: 1024 * 1024 * 1024 * 1024, // 1TB
          used: 650 * 1024 * 1024 * 1024, // 650GB
          free: 374 * 1024 * 1024 * 1024, // 374GB
          databases: [
            { name: 'production', size: 400 * 1024 * 1024 * 1024, color: '#3b82f6' },
            { name: 'analytics', size: 150 * 1024 * 1024 * 1024, color: '#10b981' },
            { name: 'staging', size: 75 * 1024 * 1024 * 1024, color: '#f59e0b' },
            { name: 'development', size: 25 * 1024 * 1024 * 1024, color: '#8b5cf6' },
          ],
        },
        bufferPool: {
          totalSize: 16 * 1024 * 1024 * 1024, // 16GB
          usedSize: 14.2 * 1024 * 1024 * 1024, // 14.2GB
          dirtyPages: 1250,
          hitRatio: 98.5,
          evictions: 320,
        },
        ioMetrics: {
          readOps: 1250,
          writeOps: 450,
          readBytes: 125 * 1024 * 1024,
          writeBytes: 45 * 1024 * 1024,
          avgReadLatency: 0.8,
          avgWriteLatency: 1.2,
        },
        tablespaces: [
          {
            id: '1',
            name: 'primary_data',
            location: '/var/lib/rustydb/data',
            size: 500 * 1024 * 1024 * 1024,
            used: 350 * 1024 * 1024 * 1024,
            status: 'online',
            tableCount: 45,
          },
          {
            id: '2',
            name: 'indexes',
            location: '/var/lib/rustydb/indexes',
            size: 200 * 1024 * 1024 * 1024,
            used: 120 * 1024 * 1024 * 1024,
            status: 'online',
            tableCount: 28,
          },
          {
            id: '3',
            name: 'archive',
            location: '/mnt/archive/rustydb',
            size: 300 * 1024 * 1024 * 1024,
            used: 180 * 1024 * 1024 * 1024,
            status: 'readonly',
            tableCount: 12,
          },
        ],
        partitions: [
          {
            id: '1',
            tableName: 'transactions',
            partitionName: 'transactions_2024_q4',
            strategy: 'range',
            size: 25 * 1024 * 1024 * 1024,
            rowCount: 5000000,
            lastAccessed: '2024-12-11T10:30:00Z',
          },
          {
            id: '2',
            tableName: 'logs',
            partitionName: 'logs_december',
            strategy: 'range',
            size: 15 * 1024 * 1024 * 1024,
            rowCount: 12000000,
            lastAccessed: '2024-12-11T10:25:00Z',
          },
          {
            id: '3',
            tableName: 'users_shard',
            partitionName: 'users_0_1000',
            strategy: 'hash',
            size: 8 * 1024 * 1024 * 1024,
            rowCount: 1000,
            lastAccessed: '2024-12-11T09:15:00Z',
          },
        ],
      };
      setStatus(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load storage data');
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

  const formatNumber = (num: number): string => {
    return num.toLocaleString();
  };

  if (loading && !status) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading storage data...</div>
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

  const diskUsagePercent = (status.diskUsage.used / status.diskUsage.total) * 100;
  const bufferPoolPercent = (status.bufferPool.usedSize / status.bufferPool.totalSize) * 100;

  const pieData = status.diskUsage.databases.map(db => ({
    name: db.name,
    value: db.size,
    color: db.color,
  }));

  const ioChartData = [
    {
      name: 'Read',
      ops: status.ioMetrics.readOps,
      bytes: status.ioMetrics.readBytes / (1024 * 1024),
      latency: status.ioMetrics.avgReadLatency,
    },
    {
      name: 'Write',
      ops: status.ioMetrics.writeOps,
      bytes: status.ioMetrics.writeBytes / (1024 * 1024),
      latency: status.ioMetrics.avgWriteLatency,
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Storage Management</h1>
          <p className="text-gray-400 mt-1">Monitor disk usage, buffer pool, and partition performance</p>
        </div>
        <div className="flex items-center space-x-3">
          <select
            value={refreshInterval}
            onChange={(e) => setRefreshInterval(Number(e.target.value))}
            className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value={10000}>Refresh: 10s</option>
            <option value={30000}>Refresh: 30s</option>
            <option value={60000}>Refresh: 1m</option>
            <option value={300000}>Refresh: 5m</option>
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

      {/* Tabs */}
      <div className="flex space-x-2 border-b border-gray-700">
        {(['overview', 'tablespaces', 'partitions'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setSelectedTab(tab)}
            className={`px-4 py-2 font-medium capitalize transition-colors ${
              selectedTab === tab
                ? 'text-blue-400 border-b-2 border-blue-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* Overview Tab */}
      {selectedTab === 'overview' && (
        <>
          {/* Key Metrics */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-medium text-gray-400">Total Disk Space</h3>
                <CircleStackIcon className="h-6 w-6 text-blue-400" />
              </div>
              <div className="text-3xl font-bold text-white">{formatBytes(status.diskUsage.total)}</div>
              <div className="text-xs text-gray-500 mt-1">
                {formatBytes(status.diskUsage.used)} used ({diskUsagePercent.toFixed(1)}%)
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-medium text-gray-400">Buffer Pool</h3>
                <ServerIcon className="h-6 w-6 text-green-400" />
              </div>
              <div className="text-3xl font-bold text-white">{formatBytes(status.bufferPool.totalSize)}</div>
              <div className="text-xs text-gray-500 mt-1">
                {formatBytes(status.bufferPool.usedSize)} used ({bufferPoolPercent.toFixed(1)}%)
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.2 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-medium text-gray-400">Cache Hit Ratio</h3>
                <ChartPieIcon className="h-6 w-6 text-purple-400" />
              </div>
              <div className="text-3xl font-bold text-white">{status.bufferPool.hitRatio}%</div>
              <div className="text-xs text-gray-500 mt-1">
                {formatNumber(status.bufferPool.evictions)} evictions
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.3 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-medium text-gray-400">I/O Operations</h3>
                <CpuChipIcon className="h-6 w-6 text-orange-400" />
              </div>
              <div className="text-3xl font-bold text-white">
                {formatNumber(status.ioMetrics.readOps + status.ioMetrics.writeOps)}
              </div>
              <div className="text-xs text-gray-500 mt-1">
                {formatNumber(status.ioMetrics.readOps)} read / {formatNumber(status.ioMetrics.writeOps)} write
              </div>
            </motion.div>
          </div>

          {/* Charts Row */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Disk Usage Pie Chart */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.4 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-lg font-semibold text-white mb-4">Disk Usage by Database</h3>
              <ResponsiveContainer width="100%" height={300}>
                <PieChart>
                  <Pie
                    data={pieData}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    label={({ name, percent }) => `${name} ${(percent * 100).toFixed(0)}%`}
                    outerRadius={100}
                    fill="#8884d8"
                    dataKey="value"
                  >
                    {pieData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value) => formatBytes(value as number)} />
                  <Legend />
                </PieChart>
              </ResponsiveContainer>
            </motion.div>

            {/* I/O Metrics Chart */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.5 }}
              className="bg-gray-800 rounded-lg p-6"
            >
              <h3 className="text-lg font-semibold text-white mb-4">I/O Performance</h3>
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={ioChartData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                  <XAxis dataKey="name" stroke="#9CA3AF" />
                  <YAxis stroke="#9CA3AF" />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#1F2937',
                      border: '1px solid #374151',
                      borderRadius: '0.5rem',
                    }}
                  />
                  <Legend />
                  <Bar dataKey="ops" fill="#3b82f6" name="Operations/sec" />
                  <Bar dataKey="latency" fill="#10b981" name="Latency (ms)" />
                </BarChart>
              </ResponsiveContainer>
            </motion.div>
          </div>

          {/* Buffer Pool Details */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.6 }}
            className="bg-gray-800 rounded-lg p-6"
          >
            <h3 className="text-lg font-semibold text-white mb-4">Buffer Pool Statistics</h3>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <p className="text-sm text-gray-400 mb-1">Total Size</p>
                <p className="text-2xl font-bold text-white">{formatBytes(status.bufferPool.totalSize)}</p>
              </div>
              <div>
                <p className="text-sm text-gray-400 mb-1">Dirty Pages</p>
                <p className="text-2xl font-bold text-yellow-400">{formatNumber(status.bufferPool.dirtyPages)}</p>
              </div>
              <div>
                <p className="text-sm text-gray-400 mb-1">Hit Ratio</p>
                <p className="text-2xl font-bold text-green-400">{status.bufferPool.hitRatio}%</p>
              </div>
            </div>
            <div className="mt-4 bg-gray-700 rounded-full h-4 overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-blue-500 to-green-500 transition-all"
                style={{ width: `${bufferPoolPercent}%` }}
              />
            </div>
          </motion.div>
        </>
      )}

      {/* Tablespaces Tab */}
      {selectedTab === 'tablespaces' && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-white">Tablespaces</h3>
            <Button variant="primary" size="sm" icon={<PlusIcon className="h-4 w-4" />}>
              Create Tablespace
            </Button>
          </div>
          <Table
            columns={[
              { key: 'name', header: 'Name', sortable: true },
              { key: 'location', header: 'Location', sortable: true },
              {
                key: 'size',
                header: 'Size',
                render: (value) => formatBytes(value),
                sortable: true,
              },
              {
                key: 'used',
                header: 'Used',
                render: (value, row) => {
                  const percent = (value / row.size) * 100;
                  return (
                    <div>
                      <div>{formatBytes(value)}</div>
                      <div className="text-xs text-gray-500">{percent.toFixed(1)}%</div>
                    </div>
                  );
                },
              },
              {
                key: 'status',
                header: 'Status',
                render: (value) => (
                  <StatusBadge
                    status={value === 'online' ? 'active' : value === 'readonly' ? 'warning' : 'inactive'}
                    label={value}
                  />
                ),
              },
              { key: 'tableCount', header: 'Tables', sortable: true },
              {
                key: 'actions',
                header: 'Actions',
                render: (_, row) => (
                  <div className="flex space-x-2">
                    <button className="p-1 text-blue-400 hover:text-blue-300">
                      <PencilIcon className="h-4 w-4" />
                    </button>
                    <button className="p-1 text-red-400 hover:text-red-300">
                      <TrashIcon className="h-4 w-4" />
                    </button>
                  </div>
                ),
              },
            ]}
            data={status.tablespaces}
            striped
            hoverable
          />
        </motion.div>
      )}

      {/* Partitions Tab */}
      {selectedTab === 'partitions' && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-6"
        >
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-white">Table Partitions</h3>
            <Button
              variant="primary"
              size="sm"
              icon={<PlusIcon className="h-4 w-4" />}
              onClick={() => setShowPartitionModal(true)}
            >
              Create Partition
            </Button>
          </div>
          <Table
            columns={[
              { key: 'tableName', header: 'Table', sortable: true },
              { key: 'partitionName', header: 'Partition', sortable: true },
              {
                key: 'strategy',
                header: 'Strategy',
                render: (value) => <Badge variant="info">{value}</Badge>,
              },
              {
                key: 'size',
                header: 'Size',
                render: (value) => formatBytes(value),
                sortable: true,
              },
              {
                key: 'rowCount',
                header: 'Rows',
                render: (value) => formatNumber(value),
                sortable: true,
              },
              {
                key: 'lastAccessed',
                header: 'Last Accessed',
                render: (value) => new Date(value).toLocaleString(),
                sortable: true,
              },
              {
                key: 'actions',
                header: 'Actions',
                render: () => (
                  <div className="flex space-x-2">
                    <button className="p-1 text-blue-400 hover:text-blue-300">
                      <PencilIcon className="h-4 w-4" />
                    </button>
                    <button className="p-1 text-red-400 hover:text-red-300">
                      <TrashIcon className="h-4 w-4" />
                    </button>
                  </div>
                ),
              },
            ]}
            data={status.partitions}
            striped
            hoverable
          />
        </motion.div>
      )}

      {/* Storage Health Info */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.7 }}
        className={`border rounded-lg p-4 ${
          diskUsagePercent > 90
            ? 'bg-red-900 bg-opacity-20 border-red-500'
            : diskUsagePercent > 75
            ? 'bg-yellow-900 bg-opacity-20 border-yellow-500'
            : 'bg-blue-900 bg-opacity-20 border-blue-500 border-opacity-30'
        }`}
      >
        <h3 className="text-sm font-semibold mb-2">
          {diskUsagePercent > 90 ? '⚠️ Storage Warning' : diskUsagePercent > 75 ? '⚡ Storage Alert' : '💡 Storage Tips'}
        </h3>
        <ul className="text-xs space-y-1 list-disc list-inside">
          {diskUsagePercent > 90 ? (
            <>
              <li>Disk usage is critical ({diskUsagePercent.toFixed(1)}%). Consider expanding storage.</li>
              <li>Archive or delete old data to free up space.</li>
            </>
          ) : diskUsagePercent > 75 ? (
            <>
              <li>Disk usage is high ({diskUsagePercent.toFixed(1)}%). Monitor closely.</li>
              <li>Review large tables and consider partitioning strategies.</li>
            </>
          ) : (
            <>
              <li>Buffer pool hit ratio of {status.bufferPool.hitRatio}% indicates good memory performance.</li>
              <li>Maintain regular vacuum operations to optimize storage.</li>
              <li>Consider using tablespaces to distribute I/O across multiple disks.</li>
            </>
          )}
        </ul>
      </motion.div>

      {/* Partition Modal */}
      {showPartitionModal && (
        <Modal
          isOpen={showPartitionModal}
          onClose={() => setShowPartitionModal(false)}
          title="Create Partition"
          size="lg"
        >
          <div className="space-y-4">
            <Input label="Table Name" placeholder="Enter table name" />
            <Input label="Partition Name" placeholder="Enter partition name" />
            <Select
              label="Partition Strategy"
              options={[
                { value: 'range', label: 'Range' },
                { value: 'hash', label: 'Hash' },
                { value: 'list', label: 'List' },
              ]}
            />
            <Input label="Partition Key" placeholder="Enter partition key column" />
            <div className="flex justify-end space-x-3 mt-6">
              <Button variant="secondary" onClick={() => setShowPartitionModal(false)}>
                Cancel
              </Button>
              <Button variant="primary" onClick={() => setShowPartitionModal(false)}>
                Create
              </Button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
