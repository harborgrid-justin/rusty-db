// ============================================================================
// Replication Management Page
// Monitor and manage replication across cluster nodes
// ============================================================================

import { useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import { subHours } from 'date-fns';
import {
  ArrowPathIcon,
  PauseIcon,
  PlayIcon,
  Cog6ToothIcon,
  ClockIcon,
  CheckCircleIcon,
  ExclamationCircleIcon,
} from '@heroicons/react/24/outline';
import { Dialog } from '@headlessui/react';
import { useForm } from 'react-hook-form';
import { ReplicationStatus as ReplicationStatusComponent } from '../components/cluster/ReplicationStatus';
import { ReplicationLagChart } from '../components/cluster/ReplicationLagChart';
import {
  useReplicationStatus,
  useReplicationLag,
  useReplicationConfig,
  useUpdateReplicationConfig,
  usePauseReplication,
  useResumeReplication,
  useResyncNode,
  useNodes,
} from '../hooks/useCluster';
import type { ReplicationConfigUpdate } from '../services/clusterService';
import clsx from 'clsx';

export default function Replication() {
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [showConfigDialog, setShowConfigDialog] = useState(false);
  const [timeRange] = useState<{ start: string; end: string }>({
    start: subHours(new Date(), 24).toISOString(),
    end: new Date().toISOString(),
  });

  // Data hooks
  const { data: replicationStatuses = [], isLoading: statusLoading } = useReplicationStatus();
  const { data: nodes = [] } = useNodes();
  const { data: config, isLoading: configLoading } = useReplicationConfig();
  const { data: lagData = [] } = useReplicationLag(
    selectedNodeId || '',
    selectedNodeId ? timeRange : undefined
  );

  // Mutation hooks
  const updateConfigMutation = useUpdateReplicationConfig();
  const pauseMutation = usePauseReplication();
  const resumeMutation = useResumeReplication();
  const resyncMutation = useResyncNode();

  const selectedNode = nodes.find((n) => n.id === selectedNodeId);

  // Calculate stats
  const stats = useMemo(() => {
    const streaming = replicationStatuses.filter((s) => s.status === 'streaming').length;
    const catchup = replicationStatuses.filter((s) => s.status === 'catchup').length;
    const stopped = replicationStatuses.filter((s) => s.status === 'stopped').length;
    const errors = replicationStatuses.filter((s) => s.status === 'error').length;

    const lags = replicationStatuses
      .filter((s) => s.status === 'streaming' || s.status === 'catchup')
      .map((s) => s.lag);

    const avgLag = lags.length > 0
      ? lags.reduce((sum, lag) => sum + lag, 0) / lags.length
      : 0;

    return {
      total: replicationStatuses.length,
      streaming,
      catchup,
      stopped,
      errors,
      avgLag,
    };
  }, [replicationStatuses]);

  function handlePauseReplication(nodeId: string) {
    if (confirm('Are you sure you want to pause replication for this node?')) {
      pauseMutation.mutate(nodeId);
    }
  }

  function handleResumeReplication(nodeId: string) {
    resumeMutation.mutate(nodeId);
  }

  function handleResync(nodeId: string) {
    if (confirm('Are you sure you want to resync this node? This may take some time.')) {
      resyncMutation.mutate(nodeId);
    }
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-gray-900 flex items-center space-x-3">
              <ArrowPathIcon className="w-8 h-8 text-blue-600" />
              <span>Replication Management</span>
            </h1>
            <p className="text-gray-600 mt-1">
              Monitor replication status and lag across all nodes
            </p>
          </div>

          <button
            onClick={() => setShowConfigDialog(true)}
            className="flex items-center space-x-2 px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors"
          >
            <Cog6ToothIcon className="w-5 h-5" />
            <span>Configure</span>
          </button>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-5 gap-4">
          <div className="bg-gray-50 rounded-lg p-4 border border-gray-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-gray-600">Total Replicas</div>
                <div className="text-2xl font-bold text-gray-900 mt-1">
                  {stats.total}
                </div>
              </div>
              <ArrowPathIcon className="w-8 h-8 text-gray-400" />
            </div>
          </div>

          <div className="bg-green-50 rounded-lg p-4 border border-green-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-green-700">Streaming</div>
                <div className="text-2xl font-bold text-green-900 mt-1">
                  {stats.streaming}
                </div>
              </div>
              <CheckCircleIcon className="w-8 h-8 text-green-500" />
            </div>
          </div>

          <div className="bg-blue-50 rounded-lg p-4 border border-blue-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-blue-700">Catching Up</div>
                <div className="text-2xl font-bold text-blue-900 mt-1">
                  {stats.catchup}
                </div>
              </div>
              <ClockIcon className="w-8 h-8 text-blue-500" />
            </div>
          </div>

          <div className="bg-gray-50 rounded-lg p-4 border border-gray-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-gray-600">Stopped</div>
                <div className="text-2xl font-bold text-gray-900 mt-1">
                  {stats.stopped}
                </div>
              </div>
              <PauseIcon className="w-8 h-8 text-gray-400" />
            </div>
          </div>

          <div className="bg-amber-50 rounded-lg p-4 border border-amber-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-amber-700">Avg Lag</div>
                <div className="text-2xl font-bold text-amber-900 mt-1">
                  {(stats.avgLag / 1000).toFixed(2)}s
                </div>
              </div>
              <ClockIcon className="w-8 h-8 text-amber-500" />
            </div>
          </div>
        </div>

        {/* Configuration Info */}
        {config && (
          <div className="mt-4 p-3 bg-blue-50 border border-blue-200 rounded-lg">
            <div className="flex items-center justify-between text-sm">
              <div className="flex items-center space-x-4">
                <div>
                  <span className="text-blue-700 font-medium">Mode: </span>
                  <span className="text-blue-900 capitalize">
                    {config.mode.replace('_', ' ')}
                  </span>
                </div>
                <div>
                  <span className="text-blue-700 font-medium">WAL Level: </span>
                  <span className="text-blue-900">{config.walLevel}</span>
                </div>
                <div>
                  <span className="text-blue-700 font-medium">Max WAL Senders: </span>
                  <span className="text-blue-900">{config.maxWalSenders}</span>
                </div>
              </div>
              {config.syncStandbyNames && config.syncStandbyNames.length > 0 && (
                <div className="text-xs text-blue-700">
                  Sync Standbys: {config.syncStandbyNames.join(', ')}
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6">
        {statusLoading ? (
          <div className="flex items-center justify-center h-full">
            <ArrowPathIcon className="w-8 h-8 text-blue-500 animate-spin" />
          </div>
        ) : (
          <div className="space-y-6">
            {/* Replication Status Cards */}
            <div>
              <h2 className="text-lg font-semibold text-gray-900 mb-4">
                Replication Status
              </h2>

              {replicationStatuses.length === 0 ? (
                <div className="bg-white border border-gray-200 rounded-lg p-12 text-center">
                  <ArrowPathIcon className="w-12 h-12 text-gray-400 mx-auto mb-4" />
                  <p className="text-gray-600">No replication connections found</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                  {replicationStatuses.map((status) => (
                    <motion.div
                      key={`${status.sourceNode}-${status.targetNode}`}
                      initial={{ opacity: 0, y: 20 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ duration: 0.2 }}
                      onClick={() => setSelectedNodeId(status.targetNode)}
                      className="cursor-pointer"
                    >
                      <ReplicationStatusComponent
                        status={status}
                        sourceNode={nodes.find((n) => n.id === status.sourceNode)}
                        targetNode={nodes.find((n) => n.id === status.targetNode)}
                      />

                      {/* Actions */}
                      <div className="mt-2 flex space-x-2">
                        {status.status === 'streaming' && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handlePauseReplication(status.targetNode);
                            }}
                            className="flex-1 px-3 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
                          >
                            <PauseIcon className="w-4 h-4 inline mr-1" />
                            Pause
                          </button>
                        )}

                        {status.status === 'stopped' && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleResumeReplication(status.targetNode);
                            }}
                            className="flex-1 px-3 py-2 text-sm font-medium text-green-700 bg-green-50 rounded-lg hover:bg-green-100 transition-colors"
                          >
                            <PlayIcon className="w-4 h-4 inline mr-1" />
                            Resume
                          </button>
                        )}

                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            handleResync(status.targetNode);
                          }}
                          className="flex-1 px-3 py-2 text-sm font-medium text-blue-700 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors"
                        >
                          <ArrowPathIcon className="w-4 h-4 inline mr-1" />
                          Resync
                        </button>
                      </div>
                    </motion.div>
                  ))}
                </div>
              )}
            </div>

            {/* Lag Chart */}
            {selectedNodeId && (
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.3 }}
              >
                <ReplicationLagChart
                  data={lagData}
                  nodeName={selectedNode?.name}
                />
              </motion.div>
            )}
          </div>
        )}
      </div>

      {/* Configuration Dialog */}
      <ConfigurationDialog
        isOpen={showConfigDialog}
        onClose={() => setShowConfigDialog(false)}
        config={config}
        onUpdate={(updates) => updateConfigMutation.mutate(updates)}
        isLoading={updateConfigMutation.isPending}
      />
    </div>
  );
}

// Configuration Dialog Component
function ConfigurationDialog({
  isOpen,
  onClose,
  config,
  onUpdate,
  isLoading,
}: {
  isOpen: boolean;
  onClose: () => void;
  config: any;
  onUpdate: (config: ReplicationConfigUpdate) => void;
  isLoading: boolean;
}) {
  const { register, handleSubmit, reset } = useForm<ReplicationConfigUpdate>({
    defaultValues: config
      ? {
          mode: config.mode,
          maxWalSenders: config.maxWalSenders,
          walKeepSegments: config.walKeepSegments,
        }
      : {},
  });

  function onSubmit(data: ReplicationConfigUpdate) {
    onUpdate(data);
    onClose();
    reset();
  }

  return (
    <Dialog open={isOpen} onClose={onClose} className="relative z-50">
      <div className="fixed inset-0 bg-black/30" aria-hidden="true" />

      <div className="fixed inset-0 flex items-center justify-center p-4">
        <Dialog.Panel className="mx-auto max-w-lg w-full bg-white rounded-xl shadow-2xl">
          <div className="px-6 py-4 border-b border-gray-200">
            <Dialog.Title className="text-lg font-semibold text-gray-900">
              Replication Configuration
            </Dialog.Title>
          </div>

          <form onSubmit={handleSubmit(onSubmit)} className="p-6 space-y-4">
            {/* Replication Mode */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Replication Mode
              </label>
              <select
                {...register('mode')}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="asynchronous">Asynchronous</option>
                <option value="synchronous">Synchronous</option>
                <option value="semi_synchronous">Semi-Synchronous</option>
              </select>
              <p className="mt-1 text-xs text-gray-500">
                Synchronous mode ensures data consistency but may impact performance
              </p>
            </div>

            {/* Max WAL Senders */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Max WAL Senders
              </label>
              <input
                {...register('maxWalSenders', { valueAsNumber: true })}
                type="number"
                min="0"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
              <p className="mt-1 text-xs text-gray-500">
                Maximum number of concurrent replication connections
              </p>
            </div>

            {/* WAL Keep Segments */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                WAL Keep Segments
              </label>
              <input
                {...register('walKeepSegments', { valueAsNumber: true })}
                type="number"
                min="0"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
              <p className="mt-1 text-xs text-gray-500">
                Number of WAL segments to keep for replication
              </p>
            </div>

            {/* Actions */}
            <div className="flex justify-end space-x-3 pt-4 border-t border-gray-200">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 text-gray-700 bg-gray-200 rounded-lg hover:bg-gray-300 transition-colors"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isLoading}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50"
              >
                {isLoading ? 'Saving...' : 'Save Changes'}
              </button>
            </div>
          </form>
        </Dialog.Panel>
      </div>
    </Dialog>
  );
}
