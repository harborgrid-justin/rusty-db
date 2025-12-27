// ============================================================================
// Failover Wizard Component
// Multi-step wizard for manual failover operations
// ============================================================================

import { useState, useEffect } from 'react';
import { Dialog } from '@headlessui/react';
import {
  XMarkIcon,
  CheckCircleIcon,
  ExclamationTriangleIcon,
  ArrowPathIcon,
  ClockIcon,
  ServerIcon,
} from '@heroicons/react/24/outline';
import { StarIcon } from '@heroicons/react/24/solid';
import type { ClusterNode, UUID } from '../../types';
import type { FailoverPreflightCheck } from '../../services/clusterService';
import clsx from 'clsx';

interface FailoverWizardProps {
  isOpen: boolean;
  onClose: () => void;
  currentLeader?: ClusterNode;
  nodes: ClusterNode[];
  onPreflightCheck: (nodeId: UUID) => Promise<FailoverPreflightCheck>;
  onTriggerFailover: (nodeId: UUID, force: boolean) => Promise<void>;
}

type WizardStep = 'select' | 'preflight' | 'confirm' | 'executing' | 'complete' | 'error';

export function FailoverWizard({
  isOpen,
  onClose,
  currentLeader,
  nodes,
  onPreflightCheck,
  onTriggerFailover,
}: FailoverWizardProps) {
  const [step, setStep] = useState<WizardStep>('select');
  const [selectedNodeId, setSelectedNodeId] = useState<UUID | null>(null);
  const [preflightResults, setPreflightResults] = useState<FailoverPreflightCheck | null>(null);
  const [forceFailover, setForceFailover] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);

  // Reset state when dialog opens/closes
  useEffect(() => {
    if (!isOpen) {
      setTimeout(() => {
        setStep('select');
        setSelectedNodeId(null);
        setPreflightResults(null);
        setForceFailover(false);
        setError(null);
        setProgress(0);
      }, 300);
    }
  }, [isOpen]);

  // Filter eligible nodes (exclude current leader and unhealthy nodes)
  const eligibleNodes = nodes.filter(
    (node) =>
      node.id !== currentLeader?.id &&
      (node.role === 'follower' || node.role === 'candidate') &&
      node.status !== 'failed' &&
      node.status !== 'unreachable'
  );

  const selectedNode = eligibleNodes.find((n) => n.id === selectedNodeId);

  async function handleNodeSelect(nodeId: UUID) {
    setSelectedNodeId(nodeId);
    setStep('preflight');
    setError(null);

    try {
      const results = await onPreflightCheck(nodeId);
      setPreflightResults(results);
      setStep('confirm');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to run preflight checks');
      setStep('error');
    }
  }

  async function handleConfirm() {
    if (!selectedNodeId) return;

    setStep('executing');
    setError(null);
    setProgress(0);

    // Simulate progress
    const progressInterval = setInterval(() => {
      setProgress((prev) => Math.min(prev + 10, 90));
    }, 500);

    try {
      await onTriggerFailover(selectedNodeId, forceFailover);
      clearInterval(progressInterval);
      setProgress(100);
      setStep('complete');
    } catch (err) {
      clearInterval(progressInterval);
      setError(err instanceof Error ? err.message : 'Failover failed');
      setStep('error');
    }
  }

  function handleClose() {
    if (step === 'executing') return; // Don't allow closing during execution
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
              <StarIcon className="w-6 h-6 text-amber-500" />
              <span>Manual Failover Wizard</span>
            </Dialog.Title>
            {step !== 'executing' && (
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
                label="Select Node"
                active={step === 'select'}
                completed={['preflight', 'confirm', 'executing', 'complete'].includes(step)}
              />
              <div className="flex-1 h-0.5 bg-gray-300 mx-2" />
              <StepIndicator
                label="Preflight Check"
                active={step === 'preflight'}
                completed={['confirm', 'executing', 'complete'].includes(step)}
              />
              <div className="flex-1 h-0.5 bg-gray-300 mx-2" />
              <StepIndicator
                label="Confirm"
                active={step === 'confirm'}
                completed={['executing', 'complete'].includes(step)}
              />
              <div className="flex-1 h-0.5 bg-gray-300 mx-2" />
              <StepIndicator
                label="Execute"
                active={step === 'executing'}
                completed={step === 'complete'}
              />
            </div>
          </div>

          {/* Content */}
          <div className="px-6 py-6 min-h-[400px]">
            {/* Step: Select Node */}
            {step === 'select' && (
              <div>
                <h3 className="text-lg font-semibold text-gray-900 mb-4">
                  Select Target Node
                </h3>

                {currentLeader && (
                  <div className="mb-4 p-4 bg-amber-50 border border-amber-200 rounded-lg">
                    <div className="flex items-center space-x-2 text-amber-900">
                      <StarIcon className="w-5 h-5" />
                      <span className="font-medium">Current Leader: {currentLeader.name}</span>
                    </div>
                    <p className="text-sm text-amber-700 mt-1">
                      {currentLeader.host}:{currentLeader.port}
                    </p>
                  </div>
                )}

                <div className="space-y-3">
                  {eligibleNodes.length === 0 ? (
                    <div className="text-center py-8 text-gray-500">
                      <ServerIcon className="w-12 h-12 mx-auto text-gray-400 mb-2" />
                      <p>No eligible nodes available for failover</p>
                    </div>
                  ) : (
                    eligibleNodes.map((node) => (
                      <button
                        key={node.id}
                        onClick={() => handleNodeSelect(node.id)}
                        className="w-full p-4 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition-all text-left"
                      >
                        <div className="flex items-center justify-between">
                          <div>
                            <div className="font-semibold text-gray-900">{node.name}</div>
                            <div className="text-sm text-gray-600">
                              {node.host}:{node.port}
                            </div>
                            {node.region && (
                              <div className="text-xs text-gray-500 mt-1">
                                {node.region} {node.zone && `/ ${node.zone}`}
                              </div>
                            )}
                          </div>
                          <div className="text-right">
                            <div
                              className={clsx(
                                'inline-flex items-center px-2 py-1 rounded-full text-xs font-medium',
                                node.status === 'healthy' && 'bg-green-100 text-green-800',
                                node.status === 'degraded' && 'bg-amber-100 text-amber-800'
                              )}
                            >
                              {node.status}
                            </div>
                            {node.metrics?.replicationLag && (
                              <div className="text-xs text-gray-500 mt-1">
                                Lag: {(node.metrics.replicationLag / 1000).toFixed(2)}s
                              </div>
                            )}
                          </div>
                        </div>
                      </button>
                    ))
                  )}
                </div>
              </div>
            )}

            {/* Step: Preflight Check */}
            {step === 'preflight' && (
              <div className="flex flex-col items-center justify-center h-full">
                <ArrowPathIcon className="w-12 h-12 text-blue-500 animate-spin mb-4" />
                <h3 className="text-lg font-semibold text-gray-900 mb-2">
                  Running Preflight Checks
                </h3>
                <p className="text-gray-600 text-center">
                  Verifying that {selectedNode?.name} can safely become the new leader...
                </p>
              </div>
            )}

            {/* Step: Confirm */}
            {step === 'confirm' && preflightResults && (
              <div>
                <h3 className="text-lg font-semibold text-gray-900 mb-4">
                  Confirm Failover
                </h3>

                {/* Target Node Info */}
                <div className="mb-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
                  <div className="font-medium text-blue-900 mb-2">New Leader</div>
                  <div className="text-sm text-blue-700">
                    {preflightResults.nodeName} ({preflightResults.nodeId})
                  </div>
                </div>

                {/* Preflight Results */}
                <div className="mb-4 space-y-2">
                  {preflightResults.checks.map((check, idx) => (
                    <div
                      key={idx}
                      className={clsx(
                        'flex items-start space-x-2 p-3 rounded-lg',
                        check.status === 'pass' && 'bg-green-50',
                        check.status === 'warning' && 'bg-amber-50',
                        check.status === 'fail' && 'bg-red-50'
                      )}
                    >
                      {check.status === 'pass' && (
                        <CheckCircleIcon className="w-5 h-5 text-green-600 flex-shrink-0" />
                      )}
                      {check.status === 'warning' && (
                        <ExclamationTriangleIcon className="w-5 h-5 text-amber-600 flex-shrink-0" />
                      )}
                      {check.status === 'fail' && (
                        <XMarkIcon className="w-5 h-5 text-red-600 flex-shrink-0" />
                      )}
                      <div className="flex-1">
                        <div className="text-sm font-medium text-gray-900">{check.name}</div>
                        <div className="text-sm text-gray-600">{check.message}</div>
                      </div>
                    </div>
                  ))}
                </div>

                {/* Warnings */}
                {preflightResults.warnings.length > 0 && (
                  <div className="mb-4 p-4 bg-amber-50 border border-amber-200 rounded-lg">
                    <div className="font-medium text-amber-900 mb-2">Warnings</div>
                    <ul className="space-y-1">
                      {preflightResults.warnings.map((warning, idx) => (
                        <li key={idx} className="text-sm text-amber-700">
                          • {warning}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {/* Estimated Downtime */}
                <div className="mb-4 p-4 bg-gray-50 border border-gray-200 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2">
                      <ClockIcon className="w-5 h-5 text-gray-600" />
                      <span className="text-sm font-medium text-gray-900">
                        Estimated Downtime
                      </span>
                    </div>
                    <span className="text-lg font-semibold text-gray-900">
                      {(preflightResults.estimatedDowntime / 1000).toFixed(1)}s
                    </span>
                  </div>
                </div>

                {/* Force Option */}
                {!preflightResults.canBeLeader && (
                  <label className="flex items-center space-x-2 mb-4">
                    <input
                      type="checkbox"
                      checked={forceFailover}
                      onChange={(e) => setForceFailover(e.target.checked)}
                      className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                    />
                    <span className="text-sm text-gray-700">
                      Force failover (override preflight checks)
                    </span>
                  </label>
                )}

                {/* Warning */}
                <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
                  <div className="flex items-start space-x-2">
                    <ExclamationTriangleIcon className="w-5 h-5 text-red-600 flex-shrink-0" />
                    <div>
                      <div className="text-sm font-medium text-red-900">
                        This action cannot be undone
                      </div>
                      <div className="text-sm text-red-700 mt-1">
                        Failover will cause a brief service interruption. Ensure all applications
                        can handle the leader change gracefully.
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Step: Executing */}
            {step === 'executing' && (
              <div className="flex flex-col items-center justify-center h-full">
                <ArrowPathIcon className="w-16 h-16 text-blue-500 animate-spin mb-4" />
                <h3 className="text-lg font-semibold text-gray-900 mb-2">
                  Executing Failover...
                </h3>
                <p className="text-gray-600 text-center mb-4">
                  Transferring leadership to {selectedNode?.name}
                </p>
                <div className="w-full max-w-md">
                  <div className="h-2 bg-gray-200 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-blue-500 transition-all duration-500"
                      style={{ width: `${progress}%` }}
                    />
                  </div>
                  <div className="text-center text-sm text-gray-600 mt-2">
                    {progress}% complete
                  </div>
                </div>
              </div>
            )}

            {/* Step: Complete */}
            {step === 'complete' && (
              <div className="flex flex-col items-center justify-center h-full">
                <CheckCircleIcon className="w-16 h-16 text-green-500 mb-4" />
                <h3 className="text-lg font-semibold text-gray-900 mb-2">
                  Failover Complete
                </h3>
                <p className="text-gray-600 text-center mb-4">
                  {selectedNode?.name} is now the cluster leader
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
                <XMarkIcon className="w-16 h-16 text-red-500 mb-4" />
                <h3 className="text-lg font-semibold text-gray-900 mb-2">Failover Failed</h3>
                <p className="text-gray-600 text-center mb-4">{error}</p>
                <div className="flex space-x-3">
                  <button
                    onClick={() => setStep('select')}
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

          {/* Footer Actions */}
          {step === 'confirm' && (
            <div className="flex items-center justify-between px-6 py-4 border-t border-gray-200 bg-gray-50">
              <button
                onClick={() => setStep('select')}
                className="px-4 py-2 text-gray-700 bg-gray-200 rounded-lg hover:bg-gray-300 transition-colors"
              >
                Back
              </button>
              <button
                onClick={handleConfirm}
                disabled={!preflightResults?.canBeLeader && !forceFailover}
                className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Trigger Failover
              </button>
            </div>
          )}
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
