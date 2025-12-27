// ============================================================================
// Failover Management Page
// Monitor and trigger manual failover operations
// ============================================================================

import { useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import { format, formatDistanceToNow } from 'date-fns';
import {
  ArrowPathIcon,
  ExclamationTriangleIcon,
  CheckCircleIcon,
  XCircleIcon,
  Cog6ToothIcon,
  BoltIcon,
  ClockIcon,
} from '@heroicons/react/24/outline';
import { StarIcon } from '@heroicons/react/24/solid';
import { Dialog } from '@headlessui/react';
import { useForm } from 'react-hook-form';
import { FailoverWizard } from '../components/cluster/FailoverWizard';
import {
  useNodes,
  useFailoverHistory,
  useFailoverConfig,
  useUpdateFailoverConfig,
  usePreflightFailoverCheck,
  useTriggerFailover,
} from '../hooks/useCluster';
import clsx from 'clsx';

export default function Failover() {
  const [showFailoverWizard, setShowFailoverWizard] = useState(false);
  const [showConfigDialog, setShowConfigDialog] = useState(false);

  // Data hooks
  const { data: nodes = [], isLoading: nodesLoading } = useNodes();
  const { data: history = [], isLoading: historyLoading } = useFailoverHistory(20);
  const { data: config, isLoading: configLoading } = useFailoverConfig();

  // Mutation hooks
  const updateConfigMutation = useUpdateFailoverConfig();
  const preflightCheckMutation = usePreflightFailoverCheck();
  const triggerFailoverMutation = useTriggerFailover();

  const isLoading = nodesLoading || historyLoading || configLoading;

  // Find current leader
  const currentLeader = nodes.find((node) => node.role === 'leader');

  // Calculate stats from history
  const stats = useMemo(() => {
    const last30Days = history.filter(
      (event) =>
        new Date(event.timestamp) > new Date(Date.now() - 30 * 24 * 60 * 60 * 1000)
    );

    const successful = last30Days.filter((e) => e.status === 'success').length;
    const failed = last30Days.filter((e) => e.status === 'failed').length;
    const automatic = last30Days.filter((e) => e.type === 'automatic').length;
    const manual = last30Days.filter((e) => e.type === 'manual').length;

    const avgDuration =
      last30Days.length > 0
        ? last30Days.reduce((sum, e) => sum + e.duration, 0) / last30Days.length
        : 0;

    return {
      total: last30Days.length,
      successful,
      failed,
      automatic,
      manual,
      avgDuration,
    };
  }, [history]);

  async function handleTriggerFailover(nodeId: string, force: boolean) {
    await triggerFailoverMutation.mutateAsync({ targetNodeId: nodeId, force });
  }

  async function handlePreflightCheck(nodeId: string) {
    return await preflightCheckMutation.mutateAsync(nodeId);
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-gray-900 flex items-center space-x-3">
              <BoltIcon className="w-8 h-8 text-amber-600" />
              <span>Failover Management</span>
            </h1>
            <p className="text-gray-600 mt-1">
              Monitor failover events and trigger manual failovers
            </p>
          </div>

          <div className="flex items-center space-x-3">
            <button
              onClick={() => setShowConfigDialog(true)}
              className="flex items-center space-x-2 px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors"
            >
              <Cog6ToothIcon className="w-5 h-5" />
              <span>Configure</span>
            </button>

            <button
              onClick={() => setShowFailoverWizard(true)}
              disabled={!currentLeader}
              className="flex items-center space-x-2 px-4 py-2 bg-amber-600 text-white rounded-lg hover:bg-amber-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <BoltIcon className="w-5 h-5" />
              <span>Trigger Failover</span>
            </button>
          </div>
        </div>

        {/* Current Leader */}
        {currentLeader ? (
          <div className="p-4 bg-amber-50 border border-amber-200 rounded-lg">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-3">
                <StarIcon className="w-6 h-6 text-amber-600" />
                <div>
                  <div className="text-sm font-medium text-amber-900">Current Leader</div>
                  <div className="text-lg font-bold text-amber-900 mt-0.5">
                    {currentLeader.name}
                  </div>
                  <div className="text-sm text-amber-700">
                    {currentLeader.host}:{currentLeader.port}
                  </div>
                </div>
              </div>

              <div className="text-right">
                <div
                  className={clsx(
                    'inline-flex items-center px-3 py-1 rounded-full text-sm font-medium',
                    currentLeader.status === 'healthy' && 'bg-green-100 text-green-800',
                    currentLeader.status === 'degraded' && 'bg-amber-100 text-amber-800',
                    currentLeader.status === 'unreachable' && 'bg-red-100 text-red-800'
                  )}
                >
                  {currentLeader.status}
                </div>
                <div className="text-xs text-amber-700 mt-1">
                  Leader for {formatDistanceToNow(new Date(currentLeader.startTime))}
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
            <div className="flex items-center space-x-2 text-red-900">
              <ExclamationTriangleIcon className="w-5 h-5" />
              <span className="font-medium">No leader detected in the cluster</span>
            </div>
          </div>
        )}

        {/* Failover Configuration */}
        {config && (
          <div className="mt-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
            <div className="grid grid-cols-4 gap-4 text-sm">
              <div>
                <span className="text-blue-700 font-medium">Auto Failover: </span>
                <span
                  className={clsx(
                    'font-semibold',
                    config.autoFailover ? 'text-green-700' : 'text-gray-700'
                  )}
                >
                  {config.autoFailover ? 'Enabled' : 'Disabled'}
                </span>
              </div>
              <div>
                <span className="text-blue-700 font-medium">Timeout: </span>
                <span className="text-blue-900">{(config.failoverTimeout / 1000).toFixed(0)}s</span>
              </div>
              <div>
                <span className="text-blue-700 font-medium">Health Check: </span>
                <span className="text-blue-900">
                  {(config.healthCheckInterval / 1000).toFixed(0)}s
                </span>
              </div>
              <div>
                <span className="text-blue-700 font-medium">Min Followers: </span>
                <span className="text-blue-900">{config.minHealthyFollowers}</span>
              </div>
            </div>
          </div>
        )}

        {/* Stats */}
        <div className="grid grid-cols-6 gap-4 mt-4">
          <div className="bg-gray-50 rounded-lg p-3 border border-gray-200">
            <div className="text-xs text-gray-600">Last 30 Days</div>
            <div className="text-2xl font-bold text-gray-900 mt-1">{stats.total}</div>
          </div>

          <div className="bg-green-50 rounded-lg p-3 border border-green-200">
            <div className="text-xs text-green-700">Successful</div>
            <div className="text-2xl font-bold text-green-900 mt-1">{stats.successful}</div>
          </div>

          <div className="bg-red-50 rounded-lg p-3 border border-red-200">
            <div className="text-xs text-red-700">Failed</div>
            <div className="text-2xl font-bold text-red-900 mt-1">{stats.failed}</div>
          </div>

          <div className="bg-blue-50 rounded-lg p-3 border border-blue-200">
            <div className="text-xs text-blue-700">Automatic</div>
            <div className="text-2xl font-bold text-blue-900 mt-1">{stats.automatic}</div>
          </div>

          <div className="bg-purple-50 rounded-lg p-3 border border-purple-200">
            <div className="text-xs text-purple-700">Manual</div>
            <div className="text-2xl font-bold text-purple-900 mt-1">{stats.manual}</div>
          </div>

          <div className="bg-amber-50 rounded-lg p-3 border border-amber-200">
            <div className="text-xs text-amber-700">Avg Duration</div>
            <div className="text-2xl font-bold text-amber-900 mt-1">
              {(stats.avgDuration / 1000).toFixed(1)}s
            </div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <ArrowPathIcon className="w-8 h-8 text-blue-500 animate-spin" />
          </div>
        ) : (
          <div>
            <h2 className="text-lg font-semibold text-gray-900 mb-4">Failover History</h2>

            {history.length === 0 ? (
              <div className="bg-white border border-gray-200 rounded-lg p-12 text-center">
                <ClockIcon className="w-12 h-12 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-600">No failover events recorded</p>
              </div>
            ) : (
              <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
                <table className="min-w-full divide-y divide-gray-200">
                  <thead className="bg-gray-50">
                    <tr>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        Timestamp
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        Type
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        Old Leader
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        New Leader
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        Duration
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        Status
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        Reason
                      </th>
                    </tr>
                  </thead>
                  <tbody className="bg-white divide-y divide-gray-200">
                    {history.map((event) => (
                      <motion.tr
                        key={event.id}
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        className="hover:bg-gray-50"
                      >
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                          {format(new Date(event.timestamp), 'PPpp')}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <span
                            className={clsx(
                              'inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium',
                              event.type === 'automatic' && 'bg-blue-100 text-blue-800',
                              event.type === 'manual' && 'bg-purple-100 text-purple-800',
                              event.type === 'planned' && 'bg-green-100 text-green-800'
                            )}
                          >
                            {event.type}
                          </span>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">
                          {nodes.find((n) => n.id === event.oldLeader)?.name || event.oldLeader}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 font-medium">
                          {nodes.find((n) => n.id === event.newLeader)?.name || event.newLeader}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-600">
                          {(event.duration / 1000).toFixed(2)}s
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          {event.status === 'success' ? (
                            <CheckCircleIcon className="w-5 h-5 text-green-500" />
                          ) : (
                            <XCircleIcon className="w-5 h-5 text-red-500" />
                          )}
                        </td>
                        <td className="px-6 py-4 text-sm text-gray-600">{event.reason}</td>
                      </motion.tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Failover Wizard */}
      <FailoverWizard
        isOpen={showFailoverWizard}
        onClose={() => setShowFailoverWizard(false)}
        currentLeader={currentLeader}
        nodes={nodes}
        onPreflightCheck={handlePreflightCheck}
        onTriggerFailover={handleTriggerFailover}
      />

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

interface FailoverConfig {
  autoFailover: boolean;
  failoverTimeout: number;
  healthCheckInterval: number;
  minHealthyFollowers: number;
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
  config: FailoverConfig | null | undefined;
  onUpdate: (config: FailoverConfig) => void;
  isLoading: boolean;
}) {
  const { register, handleSubmit, reset } = useForm({
    defaultValues: config
      ? {
          autoFailover: config.autoFailover,
          failoverTimeout: config.failoverTimeout / 1000, // Convert to seconds for form
          healthCheckInterval: config.healthCheckInterval / 1000,
          minHealthyFollowers: config.minHealthyFollowers,
        }
      : {},
  });

  function onSubmit(data: FailoverConfig) {
    // Convert back to milliseconds
    onUpdate({
      autoFailover: data.autoFailover,
      failoverTimeout: data.failoverTimeout * 1000,
      healthCheckInterval: data.healthCheckInterval * 1000,
      minHealthyFollowers: data.minHealthyFollowers,
    });
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
              Failover Configuration
            </Dialog.Title>
          </div>

          <form onSubmit={handleSubmit(onSubmit)} className="p-6 space-y-4">
            {/* Auto Failover */}
            <div>
              <label className="flex items-center space-x-2">
                <input
                  {...register('autoFailover')}
                  type="checkbox"
                  className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                />
                <span className="text-sm font-medium text-gray-700">
                  Enable Automatic Failover
                </span>
              </label>
              <p className="mt-1 text-xs text-gray-500 ml-6">
                Automatically promote a follower when the leader becomes unavailable
              </p>
            </div>

            {/* Failover Timeout */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Failover Timeout (seconds)
              </label>
              <input
                {...register('failoverTimeout', { valueAsNumber: true })}
                type="number"
                min="1"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
              <p className="mt-1 text-xs text-gray-500">
                Maximum time to wait for failover to complete
              </p>
            </div>

            {/* Health Check Interval */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Health Check Interval (seconds)
              </label>
              <input
                {...register('healthCheckInterval', { valueAsNumber: true })}
                type="number"
                min="1"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
              <p className="mt-1 text-xs text-gray-500">
                How often to check leader health
              </p>
            </div>

            {/* Min Healthy Followers */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Minimum Healthy Followers
              </label>
              <input
                {...register('minHealthyFollowers', { valueAsNumber: true })}
                type="number"
                min="0"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
              <p className="mt-1 text-xs text-gray-500">
                Minimum number of healthy followers required before triggering failover
              </p>
            </div>

            {/* Warning */}
            <div className="p-3 bg-amber-50 border border-amber-200 rounded-lg">
              <div className="flex items-start space-x-2">
                <ExclamationTriangleIcon className="w-5 h-5 text-amber-600 flex-shrink-0" />
                <div className="text-xs text-amber-700">
                  Changes to failover configuration will take effect immediately. Ensure you
                  understand the implications before modifying these settings.
                </div>
              </div>
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
