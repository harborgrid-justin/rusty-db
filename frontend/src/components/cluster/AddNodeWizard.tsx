// ============================================================================
// Add Node Wizard Component
// Multi-step wizard for adding new nodes to the cluster
// ============================================================================

import { useState, useEffect } from 'react';
import { Dialog } from '@headlessui/react';
import { useForm } from 'react-hook-form';
import {
  XMarkIcon,
  CheckCircleIcon,
  ExclamationCircleIcon,
  ArrowPathIcon,
  ServerIcon,
  SignalIcon,
} from '@heroicons/react/24/outline';
import type { AddNodeRequest, NodeSyncProgress } from '@/services/clusterService.ts';
import clsx from 'clsx';

interface AddNodeWizardProps {
  isOpen: boolean;
  onClose: () => void;
  onAddNode: (config: AddNodeRequest) => Promise<void>;
  onCheckProgress?: (nodeId: string) => Promise<NodeSyncProgress>;
}

type WizardStep = 'configure' | 'testing' | 'syncing' | 'complete' | 'error';

export function AddNodeWizard({
  isOpen,
  onClose,
  onAddNode,
  onCheckProgress,
}: AddNodeWizardProps) {
  const [step, setStep] = useState<WizardStep>('configure');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [syncProgress, setSyncProgress] = useState<NodeSyncProgress | null>(null);
  const [newNodeId, setNewNodeId] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    formState: { errors },
    reset,
    watch,
  } = useForm<AddNodeRequest>({
    defaultValues: {
      name: '',
      host: '',
      port: 5432,
      region: '',
      zone: '',
      initialSync: true,
    },
  });

  const initialSync = watch('initialSync');

  // Reset state when dialog opens/closes
  useEffect(() => {
    if (!isOpen) {
      setTimeout(() => {
        setStep('configure');
        setError(null);
        setSyncProgress(null);
        setNewNodeId(null);
        reset();
      }, 300);
    }
  }, [isOpen, reset]);

  // Poll sync progress
  useEffect(() => {
    if (step !== 'syncing' || !newNodeId || !onCheckProgress) return;

    const interval = setInterval(async () => {
      try {
        const progress = await onCheckProgress(newNodeId);
        setSyncProgress(progress);

        if (progress.phase === 'complete') {
          clearInterval(interval);
          setStep('complete');
        }
      } catch (err) {
        console.error('Failed to fetch sync progress:', err);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, [step, newNodeId, onCheckProgress]);

  async function onSubmit(data: AddNodeRequest) {
    setIsLoading(true);
    setError(null);
    setStep('testing');

    try {
      // Simulate connection test
      await new Promise((resolve) => setTimeout(resolve, 1500));

      // Add node
      await onAddNode(data);

      if (data.initialSync) {
        // If initial sync is enabled, wait for it
        setStep('syncing');
        // In a real implementation, we would get the node ID from the response
        setNewNodeId('new-node-id');
      } else {
        setStep('complete');
      }
    } catch (err: any) {
      setError(err.message || 'Failed to add node');
      setStep('error');
    } finally {
      setIsLoading(false);
    }
  }

  function handleClose() {
    if (step === 'testing' || step === 'syncing') return; // Don't allow closing during operation
    onClose();
  }

  return (
    <Dialog open={isOpen} onClose={handleClose} className="relative z-50">
      {/* Backdrop */}
      <div className="fixed inset-0 bg-black/30" aria-hidden="true" />

      {/* Dialog */}
      <div className="fixed inset-0 flex items-center justify-center p-4">
        <Dialog.Panel className="mx-auto max-w-2xl w-full bg-white rounded-xl shadow-2xl">
          {/* Header */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
            <Dialog.Title className="text-xl font-semibold text-gray-900 flex items-center space-x-2">
              <ServerIcon className="w-6 h-6 text-blue-600" />
              <span>Add Node to Cluster</span>
            </Dialog.Title>
            {step !== 'testing' && step !== 'syncing' && (
              <button
                onClick={handleClose}
                className="text-gray-400 hover:text-gray-600 transition-colors"
              >
                <XMarkIcon className="w-6 h-6" />
              </button>
            )}
          </div>

          {/* Progress Indicator */}
          <div className="px-6 py-4 border-b border-gray-200 bg-gray-50">
            <div className="flex items-center justify-between text-sm">
              <StepIndicator
                label="Configure"
                active={step === 'configure'}
                completed={['testing', 'syncing', 'complete'].includes(step)}
              />
              <div className="flex-1 h-0.5 bg-gray-300 mx-2" />
              <StepIndicator
                label="Testing"
                active={step === 'testing'}
                completed={['syncing', 'complete'].includes(step)}
              />
              {initialSync && (
                <>
                  <div className="flex-1 h-0.5 bg-gray-300 mx-2" />
                  <StepIndicator
                    label="Syncing"
                    active={step === 'syncing'}
                    completed={step === 'complete'}
                  />
                </>
              )}
              <div className="flex-1 h-0.5 bg-gray-300 mx-2" />
              <StepIndicator label="Complete" active={step === 'complete'} completed={false} />
            </div>
          </div>

          {/* Content */}
          <div className="px-6 py-6 min-h-[400px]">
            {/* Step: Configure */}
            {step === 'configure' && (
              <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
                <h3 className="text-lg font-semibold text-gray-900 mb-4">
                  Node Configuration
                </h3>

                {/* Node Name */}
                <div>
                  <label htmlFor="name" className="block text-sm font-medium text-gray-700 mb-1">
                    Node Name <span className="text-red-500">*</span>
                  </label>
                  <input
                    {...register('name', {
                      required: 'Node name is required',
                      pattern: {
                        value: /^[a-zA-Z0-9-_]+$/,
                        message: 'Only alphanumeric characters, hyphens, and underscores allowed',
                      },
                    })}
                    type="text"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    placeholder="node-3"
                  />
                  {errors.name && (
                    <p className="mt-1 text-sm text-red-600">{errors.name.message}</p>
                  )}
                </div>

                {/* Host */}
                <div>
                  <label htmlFor="host" className="block text-sm font-medium text-gray-700 mb-1">
                    Host <span className="text-red-500">*</span>
                  </label>
                  <input
                    {...register('host', {
                      required: 'Host is required',
                    })}
                    type="text"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    placeholder="192.168.1.100 or node3.example.com"
                  />
                  {errors.host && (
                    <p className="mt-1 text-sm text-red-600">{errors.host.message}</p>
                  )}
                </div>

                {/* Port */}
                <div>
                  <label htmlFor="port" className="block text-sm font-medium text-gray-700 mb-1">
                    Port <span className="text-red-500">*</span>
                  </label>
                  <input
                    {...register('port', {
                      required: 'Port is required',
                      min: { value: 1, message: 'Port must be between 1 and 65535' },
                      max: { value: 65535, message: 'Port must be between 1 and 65535' },
                      valueAsNumber: true,
                    })}
                    type="number"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    placeholder="5432"
                  />
                  {errors.port && (
                    <p className="mt-1 text-sm text-red-600">{errors.port.message}</p>
                  )}
                </div>

                {/* Region */}
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label htmlFor="region" className="block text-sm font-medium text-gray-700 mb-1">
                      Region
                    </label>
                    <input
                      {...register('region')}
                      type="text"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                      placeholder="us-west-2"
                    />
                  </div>

                  {/* Zone */}
                  <div>
                    <label htmlFor="zone" className="block text-sm font-medium text-gray-700 mb-1">
                      Zone
                    </label>
                    <input
                      {...register('zone')}
                      type="text"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                      placeholder="us-west-2a"
                    />
                  </div>
                </div>

                {/* Initial Sync */}
                <div>
                  <label className="flex items-center space-x-2">
                    <input
                      {...register('initialSync')}
                      type="checkbox"
                      className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                    />
                    <span className="text-sm text-gray-700">
                      Perform initial data synchronization
                    </span>
                  </label>
                  <p className="mt-1 text-xs text-gray-500 ml-6">
                    If enabled, the node will sync all existing data before joining the cluster
                  </p>
                </div>

                <div className="pt-4 border-t border-gray-200">
                  <div className="flex justify-end space-x-3">
                    <button
                      type="button"
                      onClick={handleClose}
                      className="px-4 py-2 text-gray-700 bg-gray-200 rounded-lg hover:bg-gray-300 transition-colors"
                    >
                      Cancel
                    </button>
                    <button
                      type="submit"
                      disabled={isLoading}
                      className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Add Node
                    </button>
                  </div>
                </div>
              </form>
            )}

            {/* Step: Testing */}
            {step === 'testing' && (
              <div className="flex flex-col items-center justify-center h-full">
                <ArrowPathIcon className="w-12 h-12 text-blue-500 animate-spin mb-4" />
                <h3 className="text-lg font-semibold text-gray-900 mb-2">
                  Testing Connection
                </h3>
                <p className="text-gray-600 text-center">
                  Verifying connectivity to the new node...
                </p>
              </div>
            )}

            {/* Step: Syncing */}
            {step === 'syncing' && (
              <div className="space-y-6">
                <div className="text-center">
                  <SignalIcon className="w-12 h-12 text-blue-500 mx-auto mb-4" />
                  <h3 className="text-lg font-semibold text-gray-900 mb-2">
                    Initial Synchronization
                  </h3>
                  <p className="text-gray-600">
                    Syncing data to the new node. This may take several minutes...
                  </p>
                </div>

                {syncProgress && (
                  <div className="space-y-4">
                    {/* Phase */}
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-600 mb-2">Current Phase</div>
                      <div className="text-lg font-semibold text-gray-900 capitalize">
                        {syncProgress.phase.replace('_', ' ')}
                      </div>
                    </div>

                    {/* Progress Bar */}
                    <div>
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-sm text-gray-600">Progress</span>
                        <span className="text-sm font-semibold text-gray-900">
                          {syncProgress.progress.toFixed(1)}%
                        </span>
                      </div>
                      <div className="h-2 bg-gray-200 rounded-full overflow-hidden">
                        <div
                          className="h-full bg-blue-500 transition-all duration-500"
                          style={{ width: `${syncProgress.progress}%` }}
                        />
                      </div>
                    </div>

                    {/* Stats */}
                    <div className="grid grid-cols-2 gap-4">
                      <div className="bg-gray-50 rounded-lg p-3">
                        <div className="text-xs text-gray-500 mb-1">Bytes Transferred</div>
                        <div className="text-sm font-semibold text-gray-900">
                          {formatBytes(syncProgress.bytesTransferred)} /{' '}
                          {formatBytes(syncProgress.totalBytes)}
                        </div>
                      </div>

                      {syncProgress.estimatedTimeRemaining && (
                        <div className="bg-gray-50 rounded-lg p-3">
                          <div className="text-xs text-gray-500 mb-1">Time Remaining</div>
                          <div className="text-sm font-semibold text-gray-900">
                            {formatDuration(syncProgress.estimatedTimeRemaining)}
                          </div>
                        </div>
                      )}
                    </div>

                    {/* LSN Progress */}
                    {syncProgress.currentLsn && syncProgress.targetLsn && (
                      <div className="bg-gray-50 rounded-lg p-3">
                        <div className="text-xs text-gray-500 mb-1">LSN Progress</div>
                        <div className="text-xs font-mono text-gray-700">
                          {syncProgress.currentLsn} → {syncProgress.targetLsn}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}

            {/* Step: Complete */}
            {step === 'complete' && (
              <div className="flex flex-col items-center justify-center h-full">
                <CheckCircleIcon className="w-16 h-16 text-green-500 mb-4" />
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Node Added Successfully</h3>
                <p className="text-gray-600 text-center mb-4">
                  The new node has been added to the cluster and is ready to serve requests
                </p>
                <button
                  onClick={handleClose}
                  className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                >
                  Close
                </button>
              </div>
            )}

            {/* Step: Error */}
            {step === 'error' && (
              <div className="flex flex-col items-center justify-center h-full">
                <ExclamationCircleIcon className="w-16 h-16 text-red-500 mb-4" />
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Failed to Add Node</h3>
                <p className="text-gray-600 text-center mb-4">{error}</p>
                <div className="flex space-x-3">
                  <button
                    onClick={() => setStep('configure')}
                    className="px-6 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors"
                  >
                    Try Again
                  </button>
                  <button
                    onClick={handleClose}
                    className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                  >
                    Close
                  </button>
                </div>
              </div>
            )}
          </div>
        </Dialog.Panel>
      </div>
    </Dialog>
  );
}

function StepIndicator({
  label,
  active,
  completed,
}: {
  label: string;
  active: boolean;
  completed: boolean;
}) {
  return (
    <div className="flex flex-col items-center">
      <div
        className={clsx(
          'w-8 h-8 rounded-full flex items-center justify-center text-xs font-semibold transition-colors',
          completed && 'bg-green-500 text-white',
          active && !completed && 'bg-blue-500 text-white',
          !active && !completed && 'bg-gray-300 text-gray-600'
        )}
      >
        {completed ? '✓' : ''}
      </div>
      <div
        className={clsx(
          'text-xs mt-1 whitespace-nowrap',
          active ? 'text-gray-900 font-medium' : 'text-gray-500'
        )}
      >
        {label}
      </div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${size.toFixed(2)} ${units[unitIndex]}`;
}

function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  } else {
    return `${seconds}s`;
  }
}
