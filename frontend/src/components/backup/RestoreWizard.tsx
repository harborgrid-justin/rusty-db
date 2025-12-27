import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ArrowPathIcon,
  CheckIcon,
  ExclamationTriangleIcon,
  ServerIcon,
  AdjustmentsHorizontalIcon,
  DocumentCheckIcon,
  XMarkIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import type { Backup, RestoreOptions, Timestamp, UUID } from '../../types';
import type { RestoreRequest } from '../../types';
import { PointInTimeSelector } from './PointInTimeSelector';
import { formatBytes, formatDate } from '../../utils/format';

// ============================================================================
// Types
// ============================================================================

interface RestoreWizardProps {
  backups: Backup[];
  databases?: string[];
  onRestore: (request: RestoreRequest) => Promise<void>;
  onCancel: () => void;
  initialBackupId?: UUID;
  loading?: boolean;
}

type WizardStep = 'select' | 'options' | 'target' | 'confirm';

// ============================================================================
// Step Navigation Component
// ============================================================================

interface StepIndicatorProps {
  currentStep: WizardStep;
  completedSteps: Set<WizardStep>;
}

function StepIndicator({ currentStep, completedSteps }: StepIndicatorProps) {
  const steps: Array<{ id: WizardStep; label: string; icon: React.ElementType }> = [
    { id: 'select', label: 'Select Backup', icon: DocumentCheckIcon },
    { id: 'options', label: 'Configure', icon: AdjustmentsHorizontalIcon },
    { id: 'target', label: 'Target', icon: ServerIcon },
    { id: 'confirm', label: 'Confirm', icon: CheckIcon },
  ];

  const stepIndex = steps.findIndex((s) => s.id === currentStep);

  return (
    <div className="flex items-center justify-between mb-8">
      {steps.map((step, index) => {
        const isActive = step.id === currentStep;
        const isCompleted = completedSteps.has(step.id);
        const Icon = step.icon;

        return (
          <div key={step.id} className="flex items-center flex-1">
            <div className="flex flex-col items-center">
              <div
                className={clsx(
                  'w-10 h-10 rounded-full flex items-center justify-center border-2 transition-colors',
                  isActive &&
                    'border-rusty-500 bg-rusty-500/20 text-rusty-400',
                  isCompleted &&
                    !isActive &&
                    'border-success-500 bg-success-500/20 text-success-400',
                  !isActive &&
                    !isCompleted &&
                    'border-dark-600 bg-dark-750 text-dark-400'
                )}
              >
                {isCompleted && !isActive ? (
                  <CheckIcon className="w-5 h-5" />
                ) : (
                  <Icon className="w-5 h-5" />
                )}
              </div>
              <span
                className={clsx(
                  'text-xs mt-2 font-medium',
                  isActive && 'text-rusty-400',
                  isCompleted && !isActive && 'text-success-400',
                  !isActive && !isCompleted && 'text-dark-400'
                )}
              >
                {step.label}
              </span>
            </div>
            {index < steps.length - 1 && (
              <div
                className={clsx(
                  'flex-1 h-0.5 mx-4 transition-colors',
                  index < stepIndex ? 'bg-success-500' : 'bg-dark-600'
                )}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

// ============================================================================
// RestoreWizard Component
// ============================================================================

export function RestoreWizard({
  backups,
  databases = [],
  onRestore,
  onCancel,
  initialBackupId,
  loading = false,
}: RestoreWizardProps) {
  const [currentStep, setCurrentStep] = useState<WizardStep>('select');
  const [completedSteps, setCompletedSteps] = useState<Set<WizardStep>>(new Set());

  const [selectedBackup, setSelectedBackup] = useState<Backup | null>(null);
  const [pointInTime, setPointInTime] = useState<Timestamp | undefined>();
  const [targetDatabase, setTargetDatabase] = useState<string>('');
  const [options, setOptions] = useState<RestoreOptions>({
    createDatabase: false,
    dropExisting: false,
    parallel: 4,
    includeIndexes: true,
    includeConstraints: true,
  });

  // Initialize with backup if provided
  useEffect(() => {
    if (initialBackupId) {
      const backup = backups.find((b) => b.id === initialBackupId);
      if (backup) {
        setSelectedBackup(backup);
        markStepComplete('select');
        setCurrentStep('options');
      }
    }
  }, [initialBackupId, backups]);

  const markStepComplete = (step: WizardStep) => {
    setCompletedSteps((prev) => new Set([...prev, step]));
  };

  const handleNext = () => {
    markStepComplete(currentStep);

    switch (currentStep) {
      case 'select':
        setCurrentStep('options');
        break;
      case 'options':
        setCurrentStep('target');
        break;
      case 'target':
        setCurrentStep('confirm');
        break;
    }
  };

  const handleBack = () => {
    switch (currentStep) {
      case 'options':
        setCurrentStep('select');
        break;
      case 'target':
        setCurrentStep('options');
        break;
      case 'confirm':
        setCurrentStep('target');
        break;
    }
  };

  const handleRestore = async () => {
    if (!selectedBackup) return;

    const request: RestoreRequest = {
      backupId: selectedBackup.id,
      targetDatabase: targetDatabase || undefined,
      pointInTime,
      options,
    };

    await onRestore(request);
  };

  const canProceed = () => {
    switch (currentStep) {
      case 'select':
        return selectedBackup !== null;
      case 'options':
        return true;
      case 'target':
        return true;
      case 'confirm':
        return true;
      default:
        return false;
    }
  };

  // Filter completed backups
  const availableBackups = backups.filter((b) => b.status === 'completed');

  return (
    <div className="space-y-6">
      {/* Step Indicator */}
      <StepIndicator currentStep={currentStep} completedSteps={completedSteps} />

      {/* Step Content */}
      <AnimatePresence mode="wait">
        <motion.div
          key={currentStep}
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: -20 }}
          transition={{ duration: 0.2 }}
        >
          {/* Step 1: Select Backup */}
          {currentStep === 'select' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-dark-100">
                Select Backup to Restore
              </h3>
              <p className="text-sm text-dark-400">
                Choose a completed backup to restore from
              </p>

              <div className="space-y-2">
                {availableBackups.length === 0 ? (
                  <div className="card p-8 text-center">
                    <p className="text-dark-400">No completed backups available</p>
                  </div>
                ) : (
                  availableBackups.map((backup) => (
                    <button
                      key={backup.id}
                      type="button"
                      onClick={() => setSelectedBackup(backup)}
                      className={clsx(
                        'w-full p-4 rounded-lg border-2 text-left transition-colors',
                        selectedBackup?.id === backup.id
                          ? 'border-rusty-500 bg-rusty-500/10'
                          : 'border-dark-600 hover:border-dark-500 bg-dark-750'
                      )}
                    >
                      <div className="flex items-start justify-between">
                        <div className="flex-1">
                          <div className="font-medium text-dark-100">
                            {backup.name}
                          </div>
                          <div className="text-sm text-dark-400 mt-1">
                            {backup.database && `Database: ${backup.database} • `}
                            Type: {backup.type} • Size: {formatBytes(backup.size)}
                          </div>
                          <div className="text-xs text-dark-500 mt-1">
                            Created {formatDate(backup.startTime)}
                          </div>
                        </div>
                        {selectedBackup?.id === backup.id && (
                          <CheckIcon className="w-5 h-5 text-rusty-400 flex-shrink-0" />
                        )}
                      </div>
                    </button>
                  ))
                )}
              </div>
            </div>
          )}

          {/* Step 2: Configure Options */}
          {currentStep === 'options' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-semibold text-dark-100">
                  Configure Restore Options
                </h3>
                <p className="text-sm text-dark-400 mt-1">
                  Customize how the backup will be restored
                </p>
              </div>

              {/* Point-in-Time Recovery */}
              <div className="card p-6">
                <label className="flex items-center gap-3 mb-4">
                  <input
                    type="checkbox"
                    checked={pointInTime !== undefined}
                    onChange={(e) =>
                      setPointInTime(
                        e.target.checked
                          ? new Date().toISOString()
                          : undefined
                      )
                    }
                    className="checkbox"
                  />
                  <span className="font-medium text-dark-100">
                    Enable Point-in-Time Recovery
                  </span>
                </label>

                <AnimatePresence>
                  {pointInTime !== undefined && (
                    <motion.div
                      initial={{ opacity: 0, height: 0 }}
                      animate={{ opacity: 1, height: 'auto' }}
                      exit={{ opacity: 0, height: 0 }}
                    >
                      <PointInTimeSelector
                        value={pointInTime}
                        onChange={setPointInTime}
                        maxDate={new Date().toISOString()}
                      />
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>

              {/* Restore Options */}
              <div className="card p-6 space-y-4">
                <h4 className="font-medium text-dark-100">Restore Options</h4>

                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={options.includeIndexes}
                    onChange={(e) =>
                      setOptions({ ...options, includeIndexes: e.target.checked })
                    }
                    className="checkbox"
                  />
                  <span className="text-sm text-dark-200">Include Indexes</span>
                </label>

                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={options.includeConstraints}
                    onChange={(e) =>
                      setOptions({
                        ...options,
                        includeConstraints: e.target.checked,
                      })
                    }
                    className="checkbox"
                  />
                  <span className="text-sm text-dark-200">Include Constraints</span>
                </label>

                <div>
                  <label className="block text-sm font-medium text-dark-200 mb-2">
                    Parallel Workers
                  </label>
                  <input
                    type="number"
                    value={options.parallel}
                    onChange={(e) =>
                      setOptions({
                        ...options,
                        parallel: parseInt(e.target.value) || 1,
                      })
                    }
                    min={1}
                    max={16}
                    className="input w-32"
                  />
                  <p className="text-xs text-dark-400 mt-1">
                    Number of parallel restore workers (1-16)
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Step 3: Select Target */}
          {currentStep === 'target' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-semibold text-dark-100">
                  Select Target Database
                </h3>
                <p className="text-sm text-dark-400 mt-1">
                  Choose where to restore the backup
                </p>
              </div>

              <div className="card p-6 space-y-4">
                <div>
                  <label className="block text-sm font-medium text-dark-200 mb-2">
                    Target Database Name
                  </label>
                  {databases.length > 0 ? (
                    <select
                      value={targetDatabase}
                      onChange={(e) => setTargetDatabase(e.target.value)}
                      className="input w-full"
                    >
                      <option value="">Same as source</option>
                      {databases.map((db) => (
                        <option key={db} value={db}>
                          {db}
                        </option>
                      ))}
                      <option value="__new__">Create new database...</option>
                    </select>
                  ) : (
                    <input
                      type="text"
                      value={targetDatabase}
                      onChange={(e) => setTargetDatabase(e.target.value)}
                      placeholder="Enter database name"
                      className="input w-full"
                    />
                  )}
                </div>

                {targetDatabase === '__new__' && (
                  <div>
                    <label className="block text-sm font-medium text-dark-200 mb-2">
                      New Database Name
                    </label>
                    <input
                      type="text"
                      placeholder="Enter new database name"
                      className="input w-full"
                      onChange={(e) =>
                        setTargetDatabase(e.target.value)
                      }
                    />
                  </div>
                )}

                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={options.createDatabase}
                    onChange={(e) =>
                      setOptions({ ...options, createDatabase: e.target.checked })
                    }
                    className="checkbox"
                  />
                  <span className="text-sm text-dark-200">
                    Create database if it doesn't exist
                  </span>
                </label>

                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={options.dropExisting}
                    onChange={(e) =>
                      setOptions({ ...options, dropExisting: e.target.checked })
                    }
                    className="checkbox"
                  />
                  <span className="text-sm text-dark-200">
                    Drop existing database before restore
                  </span>
                </label>

                {options.dropExisting && (
                  <div className="p-3 bg-warning-500/10 border border-warning-500/30 rounded-lg">
                    <div className="flex items-start gap-2">
                      <ExclamationTriangleIcon className="w-5 h-5 text-warning-400 flex-shrink-0" />
                      <p className="text-sm text-warning-300">
                        Warning: This will permanently delete the existing database
                        and all its data.
                      </p>
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Step 4: Confirm */}
          {currentStep === 'confirm' && selectedBackup && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-semibold text-dark-100">
                  Confirm Restore
                </h3>
                <p className="text-sm text-dark-400 mt-1">
                  Review your restore configuration before proceeding
                </p>
              </div>

              <div className="p-6 bg-warning-500/10 border border-warning-500/30 rounded-lg">
                <div className="flex items-start gap-3">
                  <ExclamationTriangleIcon className="w-6 h-6 text-warning-400 flex-shrink-0" />
                  <div>
                    <h4 className="font-medium text-warning-300 mb-2">
                      Important Notice
                    </h4>
                    <p className="text-sm text-warning-200">
                      This operation will restore data from the selected backup. Make
                      sure you have reviewed all settings carefully. This action may
                      take some time to complete.
                    </p>
                  </div>
                </div>
              </div>

              {/* Summary */}
              <div className="card p-6 space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <div className="text-xs text-dark-400 mb-1">Backup</div>
                    <div className="text-sm font-medium text-dark-100">
                      {selectedBackup.name}
                    </div>
                  </div>
                  <div>
                    <div className="text-xs text-dark-400 mb-1">Type</div>
                    <div className="text-sm font-medium text-dark-100 capitalize">
                      {selectedBackup.type}
                    </div>
                  </div>
                  <div>
                    <div className="text-xs text-dark-400 mb-1">Size</div>
                    <div className="text-sm font-medium text-dark-100">
                      {formatBytes(selectedBackup.size)}
                    </div>
                  </div>
                  <div>
                    <div className="text-xs text-dark-400 mb-1">Created</div>
                    <div className="text-sm font-medium text-dark-100">
                      {formatDate(selectedBackup.startTime)}
                    </div>
                  </div>
                  {targetDatabase && (
                    <div>
                      <div className="text-xs text-dark-400 mb-1">Target Database</div>
                      <div className="text-sm font-medium text-dark-100">
                        {targetDatabase}
                      </div>
                    </div>
                  )}
                  {pointInTime && (
                    <div>
                      <div className="text-xs text-dark-400 mb-1">
                        Point-in-Time
                      </div>
                      <div className="text-sm font-medium text-dark-100">
                        {formatDate(pointInTime)}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}
        </motion.div>
      </AnimatePresence>

      {/* Navigation */}
      <div className="flex items-center justify-between pt-6 border-t border-dark-700">
        <button
          type="button"
          onClick={currentStep === 'select' ? onCancel : handleBack}
          className="btn-secondary"
          disabled={loading}
        >
          {currentStep === 'select' ? (
            <>
              <XMarkIcon className="w-4 h-4" />
              Cancel
            </>
          ) : (
            'Back'
          )}
        </button>

        {currentStep === 'confirm' ? (
          <button
            type="button"
            onClick={handleRestore}
            className="btn-primary"
            disabled={loading || !canProceed()}
          >
            {loading ? (
              <>
                <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                Starting Restore...
              </>
            ) : (
              <>
                <ArrowPathIcon className="w-5 h-5" />
                Start Restore
              </>
            )}
          </button>
        ) : (
          <button
            type="button"
            onClick={handleNext}
            className="btn-primary"
            disabled={!canProceed()}
          >
            Next
          </button>
        )}
      </div>
    </div>
  );
}
