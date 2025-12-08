import { useState } from 'react';
import { motion } from 'framer-motion';
import { PlusIcon, ArrowPathIcon, FunnelIcon } from '@heroicons/react/24/outline';
import { Button } from '../components/common/Button';
import { Modal } from '../components/common/Modal';
import { Input } from '../components/common/Input';
import { PoolStatsCard } from '../components/resources/PoolStatsCard';
import { PoolConfigForm, PoolConfig } from '../components/resources/PoolConfigForm';
import { DeleteConfirmDialog, ConfirmDialog } from '../components/common/ConfirmDialog';
import { useResources } from '../hooks/useResources';
import { ConnectionPoolStats } from '../types';

// ============================================================================
// Connection Pools Page
// Manage database connection pools
// ============================================================================

export default function ConnectionPools() {
  const {
    pools,
    loading,
    createPool,
    updatePool,
    deletePool,
    drainPool,
    refreshPools,
  } = useResources();

  const [showCreateModal, setShowCreateModal] = useState(false);
  const [editingPool, setEditingPool] = useState<ConnectionPoolStats | null>(null);
  const [deletingPool, setDeletingPool] = useState<ConnectionPoolStats | null>(null);
  const [drainingPool, setDrainingPool] = useState<ConnectionPoolStats | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  // Filter pools
  const filteredPools = pools.filter((pool) =>
    pool.poolId.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleCreatePool = async (config: PoolConfig) => {
    await createPool(config);
    setShowCreateModal(false);
  };

  const handleUpdatePool = async (config: PoolConfig) => {
    if (editingPool) {
      await updatePool(editingPool.poolId, config);
      setEditingPool(null);
    }
  };

  const handleDeletePool = async () => {
    if (deletingPool) {
      await deletePool(deletingPool.poolId);
      setDeletingPool(null);
    }
  };

  const handleDrainPool = async () => {
    if (drainingPool) {
      await drainPool(drainingPool.poolId);
      setDrainingPool(null);
    }
  };

  // Calculate summary stats
  const totalStats = pools.reduce(
    (acc, pool) => ({
      totalConnections: acc.totalConnections + pool.totalConnections,
      activeConnections: acc.activeConnections + pool.activeConnections,
      idleConnections: acc.idleConnections + pool.idleConnections,
      waitingRequests: acc.waitingRequests + pool.waitingRequests,
    }),
    { totalConnections: 0, activeConnections: 0, idleConnections: 0, waitingRequests: 0 }
  );

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-dark-100">Connection Pools</h1>
          <p className="text-dark-400 mt-1">
            Monitor and manage database connection pools
          </p>
        </div>
        <Button
          variant="primary"
          leftIcon={<PlusIcon className="w-5 h-5" />}
          onClick={() => setShowCreateModal(true)}
        >
          Create Pool
        </Button>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-dark-800 rounded-xl p-4 border border-dark-700">
          <div className="text-sm text-dark-400 mb-1">Total Pools</div>
          <div className="text-2xl font-bold text-dark-100">{pools.length}</div>
        </div>
        <div className="bg-dark-800 rounded-xl p-4 border border-dark-700">
          <div className="text-sm text-dark-400 mb-1">Active Connections</div>
          <div className="text-2xl font-bold text-success-500">
            {totalStats.activeConnections}
          </div>
        </div>
        <div className="bg-dark-800 rounded-xl p-4 border border-dark-700">
          <div className="text-sm text-dark-400 mb-1">Idle Connections</div>
          <div className="text-2xl font-bold text-dark-400">
            {totalStats.idleConnections}
          </div>
        </div>
        <div className="bg-dark-800 rounded-xl p-4 border border-dark-700">
          <div className="text-sm text-dark-400 mb-1">Waiting Requests</div>
          <div className={`text-2xl font-bold ${
            totalStats.waitingRequests > 0 ? 'text-warning-500' : 'text-dark-400'
          }`}>
            {totalStats.waitingRequests}
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-4">
        <Input
          placeholder="Search connection pools..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="flex-1 max-w-md"
        />
        <Button variant="ghost" onClick={refreshPools} leftIcon={<ArrowPathIcon className="w-5 h-5" />}>
          Refresh
        </Button>
      </div>

      {/* Pools Grid */}
      {loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {[...Array(6)].map((_, i) => (
            <div key={i} className="h-96 bg-dark-800 rounded-xl animate-pulse" />
          ))}
        </div>
      ) : filteredPools.length === 0 ? (
        <div className="text-center py-12 bg-dark-800 rounded-xl border border-dark-700">
          <p className="text-dark-400 mb-4">
            {searchQuery
              ? 'No connection pools match your search'
              : 'No connection pools configured'}
          </p>
          <Button
            variant="primary"
            onClick={() => setShowCreateModal(true)}
            leftIcon={<PlusIcon className="w-5 h-5" />}
          >
            Create First Pool
          </Button>
        </div>
      ) : (
        <motion.div
          className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
        >
          {filteredPools.map((pool, index) => (
            <motion.div
              key={pool.poolId}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: index * 0.05 }}
              className="relative"
            >
              <PoolStatsCard stats={pool} />

              {/* Pool Actions */}
              <div className="absolute top-4 right-4 flex gap-2">
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setEditingPool(pool)}
                >
                  Configure
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setDrainingPool(pool)}
                  className="text-warning-500 hover:text-warning-400"
                >
                  Drain
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setDeletingPool(pool)}
                  className="text-danger-500 hover:text-danger-400"
                >
                  Delete
                </Button>
              </div>
            </motion.div>
          ))}
        </motion.div>
      )}

      {/* Create Modal */}
      <Modal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        title="Create Connection Pool"
        size="lg"
      >
        <PoolConfigForm
          onSubmit={handleCreatePool}
          onCancel={() => setShowCreateModal(false)}
        />
      </Modal>

      {/* Edit Modal */}
      <Modal
        isOpen={!!editingPool}
        onClose={() => setEditingPool(null)}
        title="Configure Connection Pool"
        size="lg"
      >
        {editingPool && (
          <PoolConfigForm
            config={{
              poolId: editingPool.poolId,
              minConnections: editingPool.minConnections,
              maxConnections: editingPool.maxConnections,
              connectionTimeout: 30000,
              idleTimeout: 600000,
              validationInterval: 60000,
            }}
            onSubmit={handleUpdatePool}
            onCancel={() => setEditingPool(null)}
          />
        )}
      </Modal>

      {/* Drain Confirmation */}
      <ConfirmDialog
        isOpen={!!drainingPool}
        onClose={() => setDrainingPool(null)}
        onConfirm={handleDrainPool}
        title="Drain Connection Pool"
        message={
          <div className="space-y-2">
            <p>
              Are you sure you want to drain the pool{' '}
              <span className="font-semibold text-dark-100">{drainingPool?.poolId}</span>?
            </p>
            <p className="text-sm text-dark-400">
              This will gracefully close all idle connections and prevent new connections
              from being created. Active connections will be allowed to complete.
            </p>
          </div>
        }
        confirmLabel="Drain Pool"
        variant="warning"
      />

      {/* Delete Confirmation */}
      <DeleteConfirmDialog
        isOpen={!!deletingPool}
        onClose={() => setDeletingPool(null)}
        onConfirm={handleDeletePool}
        itemName={deletingPool?.poolId || ''}
        itemType="connection pool"
        additionalWarning="All active connections will be forcefully closed."
      />
    </div>
  );
}
