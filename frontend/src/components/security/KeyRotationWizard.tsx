// ============================================================================
// Key Rotation Wizard Component
// Step-by-step wizard for rotating encryption keys
// ============================================================================

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  XMarkIcon,
  ArrowPathIcon,
  CheckCircleIcon,
  ExclamationTriangleIcon,
  ClockIcon,
  InformationCircleIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
} from '@heroicons/react/24/outline';
import {
  useEncryptionKey,
  useRotateKey,
  useKeyRotationStatus,
  useEncryptedTables,
} from '../../hooks/useSecurity';
import clsx from 'clsx';

// ============================================================================
// Component Props
// ============================================================================

interface KeyRotationWizardProps {
  keyId: string;
  onClose: () => void;
}

// ============================================================================
// Key Rotation Wizard Component
// ============================================================================

export function KeyRotationWizard({ keyId, onClose }: KeyRotationWizardProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [gracePeriodHours, setGracePeriodHours] = useState(24);
  const [notifyUsers, setNotifyUsers] = useState(true);
  const [confirmed, setConfirmed] = useState(false);
  const [rotationStarted, setRotationStarted] = useState(false);

  const { data: key, isLoading: keyLoading } = useEncryptionKey(keyId);
  const { data: encryptedTables } = useEncryptedTables(keyId);
  const rotateKey = useRotateKey();
  const { data: rotationStatus } = useKeyRotationStatus(
    rotationStarted ? keyId : null
  );

  const steps = [
    {
      title: 'Review Key',
      description: 'Review key details and rotation requirements',
    },
    {
      title: 'Impact Analysis',
      description: 'Review affected tables and estimated downtime',
    },
    {
      title: 'Configuration',
      description: 'Configure rotation parameters',
    },
    {
      title: 'Confirmation',
      description: 'Confirm and start key rotation',
    },
    {
      title: 'Progress',
      description: 'Monitor rotation progress',
    },
  ];

  const handleStartRotation = async () => {
    try {
      await rotateKey.mutateAsync({
        keyId,
        request: {
          gracePeriodHours,
          notifyUsers,
        },
      });
      setRotationStarted(true);
      setCurrentStep(4);
    } catch (error) {
      console.error('Failed to start rotation:', error);
    }
  };

  const handleNext = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    }
  };

  const handlePrevious = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const isStepValid = () => {
    switch (currentStep) {
      case 3:
        return confirmed;
      default:
        return true;
    }
  };

  if (keyLoading) {
    return (
      <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
        <div className="card max-w-3xl w-full">
          <div className="flex items-center justify-center py-8">
            <div className="w-8 h-8 border-2 border-rusty-500 border-t-transparent rounded-full animate-spin" />
          </div>
        </div>
      </div>
    );
  }

  if (!key) {
    return null;
  }

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="card max-w-3xl w-full max-h-[90vh] overflow-y-auto"
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-6 pb-4 border-b border-dark-700">
          <h2 className="text-xl font-bold text-dark-100 flex items-center gap-3">
            <ArrowPathIcon className="w-6 h-6 text-rusty-500" />
            Key Rotation Wizard
          </h2>
          <button
            onClick={onClose}
            disabled={rotationStarted && rotationStatus?.status === 'in_progress'}
            className="btn-ghost"
          >
            <XMarkIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Progress Steps */}
        <div className="mb-8">
          <div className="flex items-center justify-between">
            {steps.map((step, index) => (
              <div key={index} className="flex items-center flex-1">
                <div className="flex flex-col items-center flex-1">
                  <div
                    className={clsx(
                      'w-10 h-10 rounded-full flex items-center justify-center font-medium transition-colors',
                      index < currentStep
                        ? 'bg-success-500 text-white'
                        : index === currentStep
                        ? 'bg-rusty-500 text-white'
                        : 'bg-dark-700 text-dark-400'
                    )}
                  >
                    {index < currentStep ? (
                      <CheckCircleIcon className="w-6 h-6" />
                    ) : (
                      index + 1
                    )}
                  </div>
                  <div className="text-xs text-dark-400 mt-2 text-center">
                    {step.title}
                  </div>
                </div>
                {index < steps.length - 1 && (
                  <div
                    className={clsx(
                      'h-0.5 flex-1 mx-2 transition-colors',
                      index < currentStep ? 'bg-success-500' : 'bg-dark-700'
                    )}
                  />
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Step Content */}
        <AnimatePresence mode="wait">
          <motion.div
            key={currentStep}
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            transition={{ duration: 0.2 }}
          >
            {/* Step 0: Review Key */}
            {currentStep === 0 && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold text-dark-100">Key Information</h3>
                <div className="grid grid-cols-2 gap-4 p-4 rounded-lg bg-dark-700/50">
                  <div>
                    <p className="text-xs text-dark-400 mb-1">Key Name</p>
                    <p className="text-sm font-medium text-dark-100">{key.name}</p>
                  </div>
                  <div>
                    <p className="text-xs text-dark-400 mb-1">Algorithm</p>
                    <p className="text-sm font-medium text-dark-100">{key.algorithm}</p>
                  </div>
                  <div>
                    <p className="text-xs text-dark-400 mb-1">Current Version</p>
                    <p className="text-sm font-medium text-dark-100">v{key.version}</p>
                  </div>
                  <div>
                    <p className="text-xs text-dark-400 mb-1">Status</p>
                    <p className="text-sm font-medium text-dark-100 capitalize">
                      {key.status}
                    </p>
                  </div>
                </div>

                <div className="p-4 rounded-lg bg-blue-500/10 border border-blue-500/30">
                  <div className="flex items-start gap-3">
                    <InformationCircleIcon className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" />
                    <div>
                      <h4 className="font-medium text-blue-400 mb-1">
                        About Key Rotation
                      </h4>
                      <p className="text-sm text-dark-300">
                        Key rotation creates a new version of the encryption key and
                        re-encrypts all data encrypted with the old version. This process
                        ensures security by limiting the amount of data encrypted with a
                        single key version.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Step 1: Impact Analysis */}
            {currentStep === 1 && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold text-dark-100">Impact Analysis</h3>

                <div className="grid grid-cols-3 gap-4">
                  <div className="card">
                    <div className="text-sm text-dark-400 mb-1">Affected Tables</div>
                    <div className="text-2xl font-bold text-dark-100">
                      {encryptedTables?.length || 0}
                    </div>
                  </div>
                  <div className="card">
                    <div className="text-sm text-dark-400 mb-1">Estimated Time</div>
                    <div className="text-2xl font-bold text-dark-100">
                      {Math.ceil((encryptedTables?.length || 0) * 2.5)} min
                    </div>
                  </div>
                  <div className="card">
                    <div className="text-sm text-dark-400 mb-1">New Version</div>
                    <div className="text-2xl font-bold text-dark-100">
                      v{key.version + 1}
                    </div>
                  </div>
                </div>

                {encryptedTables && encryptedTables.length > 0 && (
                  <div>
                    <h4 className="font-medium text-dark-100 mb-3">
                      Tables to Re-encrypt
                    </h4>
                    <div className="space-y-2 max-h-64 overflow-y-auto">
                      {encryptedTables.map((table) => (
                        <div
                          key={`${table.schema}.${table.table}`}
                          className="p-3 rounded-lg bg-dark-700/50 flex items-center justify-between"
                        >
                          <div>
                            <div className="font-mono text-sm text-dark-100">
                              {table.schema}.{table.table}
                            </div>
                            <div className="text-xs text-dark-400">
                              {table.encryptedColumns.length} encrypted column(s)
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                <div className="p-4 rounded-lg bg-warning-500/10 border border-warning-500/30">
                  <div className="flex items-start gap-3">
                    <ExclamationTriangleIcon className="w-5 h-5 text-warning-400 flex-shrink-0 mt-0.5" />
                    <div>
                      <h4 className="font-medium text-warning-400 mb-1">
                        Important Notice
                      </h4>
                      <p className="text-sm text-dark-300">
                        During rotation, affected tables may experience temporary
                        performance degradation. Consider running this during off-peak
                        hours.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Step 2: Configuration */}
            {currentStep === 2 && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold text-dark-100">
                  Rotation Configuration
                </h3>

                <div>
                  <label className="label">
                    Grace Period (Hours)
                    <span className="text-dark-500 ml-2 font-normal">
                      Time to keep old key active after rotation
                    </span>
                  </label>
                  <input
                    type="number"
                    value={gracePeriodHours}
                    onChange={(e) => setGracePeriodHours(Number(e.target.value))}
                    className="input-field w-full"
                    min="1"
                    max="168"
                  />
                  <p className="text-xs text-dark-400 mt-1">
                    Recommended: 24-48 hours for production environments
                  </p>
                </div>

                <div>
                  <label className="flex items-center gap-3 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={notifyUsers}
                      onChange={(e) => setNotifyUsers(e.target.checked)}
                      className="w-4 h-4 rounded border-dark-600 text-rusty-500 focus:ring-rusty-500"
                    />
                    <span className="text-sm text-dark-100">
                      Notify users about the key rotation
                    </span>
                  </label>
                  <p className="text-xs text-dark-400 mt-1 ml-7">
                    Users with access to encrypted data will receive a notification
                  </p>
                </div>

                <div className="p-4 rounded-lg bg-dark-700/50">
                  <h4 className="font-medium text-dark-100 mb-3">Rotation Summary</h4>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-dark-400">Tables to process:</span>
                      <span className="text-dark-100">
                        {encryptedTables?.length || 0}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-dark-400">Grace period:</span>
                      <span className="text-dark-100">{gracePeriodHours} hours</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-dark-400">User notifications:</span>
                      <span className="text-dark-100">
                        {notifyUsers ? 'Enabled' : 'Disabled'}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-dark-400">Estimated duration:</span>
                      <span className="text-dark-100">
                        {Math.ceil((encryptedTables?.length || 0) * 2.5)} minutes
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Step 3: Confirmation */}
            {currentStep === 3 && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold text-dark-100">
                  Confirm Key Rotation
                </h3>

                <div className="p-4 rounded-lg bg-danger-500/10 border border-danger-500/30">
                  <div className="flex items-start gap-3">
                    <ExclamationTriangleIcon className="w-5 h-5 text-danger-400 flex-shrink-0 mt-0.5" />
                    <div>
                      <h4 className="font-medium text-danger-400 mb-1">
                        Critical Operation
                      </h4>
                      <p className="text-sm text-dark-300 mb-3">
                        This will rotate the encryption key and re-encrypt all data. This
                        operation cannot be cancelled once started.
                      </p>
                      <ul className="text-sm text-dark-300 space-y-1 list-disc list-inside">
                        <li>All encrypted data will be re-encrypted</li>
                        <li>Performance may be impacted during rotation</li>
                        <li>The old key will remain active for {gracePeriodHours} hours</li>
                        <li>Monitor the progress carefully</li>
                      </ul>
                    </div>
                  </div>
                </div>

                <div className="p-4 rounded-lg bg-dark-700/50">
                  <label className="flex items-start gap-3 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={confirmed}
                      onChange={(e) => setConfirmed(e.target.checked)}
                      className="w-4 h-4 mt-1 rounded border-dark-600 text-rusty-500 focus:ring-rusty-500"
                    />
                    <div>
                      <span className="text-sm text-dark-100 font-medium">
                        I understand and confirm this operation
                      </span>
                      <p className="text-xs text-dark-400 mt-1">
                        I have reviewed the impact and am ready to proceed with key
                        rotation for key: <span className="font-mono">{key.name}</span>
                      </p>
                    </div>
                  </label>
                </div>
              </div>
            )}

            {/* Step 4: Progress */}
            {currentStep === 4 && rotationStatus && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold text-dark-100">Rotation Progress</h3>

                <div className="p-4 rounded-lg bg-dark-700/50">
                  <div className="flex items-center justify-between mb-3">
                    <span className="text-sm text-dark-300">
                      Status: <span className="font-medium text-dark-100 capitalize">
                        {rotationStatus.status.replace('_', ' ')}
                      </span>
                    </span>
                    <span className="text-sm text-dark-300">
                      {rotationStatus.progress}% Complete
                    </span>
                  </div>
                  <div className="progress-bar h-3">
                    <motion.div
                      className={clsx(
                        'progress-fill h-full',
                        rotationStatus.status === 'completed'
                          ? 'bg-success-500'
                          : rotationStatus.status === 'failed'
                          ? 'bg-danger-500'
                          : 'bg-rusty-500'
                      )}
                      initial={{ width: 0 }}
                      animate={{ width: `${rotationStatus.progress}%` }}
                      transition={{ duration: 0.5 }}
                    />
                  </div>
                </div>

                {rotationStatus.estimatedCompletion && (
                  <div className="flex items-center gap-2 text-sm text-dark-400">
                    <ClockIcon className="w-4 h-4" />
                    <span>
                      Estimated completion:{' '}
                      {new Date(rotationStatus.estimatedCompletion).toLocaleTimeString()}
                    </span>
                  </div>
                )}

                {rotationStatus.affectedTables.length > 0 && (
                  <div>
                    <h4 className="font-medium text-dark-100 mb-3">Processing Tables</h4>
                    <div className="space-y-1 max-h-48 overflow-y-auto">
                      {rotationStatus.affectedTables.map((table, idx) => (
                        <div
                          key={idx}
                          className="text-sm font-mono text-dark-300 flex items-center gap-2"
                        >
                          <CheckCircleIcon className="w-4 h-4 text-success-400" />
                          {table}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {rotationStatus.errors && rotationStatus.errors.length > 0 && (
                  <div className="p-4 rounded-lg bg-danger-500/10 border border-danger-500/30">
                    <h4 className="font-medium text-danger-400 mb-2">Errors</h4>
                    <div className="space-y-1">
                      {rotationStatus.errors.map((error, idx) => (
                        <p key={idx} className="text-sm text-dark-300">
                          {error}
                        </p>
                      ))}
                    </div>
                  </div>
                )}

                {rotationStatus.status === 'completed' && (
                  <div className="p-4 rounded-lg bg-success-500/10 border border-success-500/30">
                    <div className="flex items-start gap-3">
                      <CheckCircleIcon className="w-5 h-5 text-success-400 flex-shrink-0 mt-0.5" />
                      <div>
                        <h4 className="font-medium text-success-400 mb-1">
                          Rotation Complete
                        </h4>
                        <p className="text-sm text-dark-300">
                          Key rotation completed successfully. The new key version is now
                          active.
                        </p>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            )}
          </motion.div>
        </AnimatePresence>

        {/* Actions */}
        <div className="flex gap-3 mt-8 pt-6 border-t border-dark-700">
          {currentStep < 4 ? (
            <>
              <button
                onClick={handlePrevious}
                disabled={currentStep === 0}
                className="btn-secondary"
              >
                <ChevronLeftIcon className="w-4 h-4" />
                Previous
              </button>
              <div className="flex-1" />
              {currentStep === 3 ? (
                <button
                  onClick={handleStartRotation}
                  disabled={!isStepValid() || rotateKey.isPending}
                  className="btn-primary"
                >
                  {rotateKey.isPending ? (
                    <>
                      <ArrowPathIcon className="w-4 h-4 animate-spin" />
                      Starting...
                    </>
                  ) : (
                    <>
                      <ArrowPathIcon className="w-4 h-4" />
                      Start Rotation
                    </>
                  )}
                </button>
              ) : (
                <button
                  onClick={handleNext}
                  disabled={!isStepValid()}
                  className="btn-primary"
                >
                  Next
                  <ChevronRightIcon className="w-4 h-4" />
                </button>
              )}
            </>
          ) : (
            <button
              onClick={onClose}
              disabled={rotationStatus?.status === 'in_progress'}
              className="btn-primary w-full"
            >
              {rotationStatus?.status === 'in_progress' ? 'Please Wait...' : 'Close'}
            </button>
          )}
        </div>
      </motion.div>
    </div>
  );
}
