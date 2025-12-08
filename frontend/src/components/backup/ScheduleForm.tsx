import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import {
  ClockIcon,
  CalendarIcon,
  BellIcon,
  ShieldCheckIcon,
  ArchiveBoxIcon,
  InformationCircleIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import type { BackupType, BackupSchedule } from '../../types';
import type { CreateScheduleConfig } from '../../services/backupService';
import { formatCronExpression } from '../../utils/format';

// ============================================================================
// Types
// ============================================================================

interface ScheduleFormProps {
  schedule?: BackupSchedule;
  onSubmit: (config: CreateScheduleConfig) => Promise<void>;
  onCancel: () => void;
  databases?: string[];
  loading?: boolean;
}

interface FrequencyPreset {
  label: string;
  cron: string;
  description: string;
}

// ============================================================================
// Frequency Presets
// ============================================================================

const FREQUENCY_PRESETS: FrequencyPreset[] = [
  {
    label: 'Every Hour',
    cron: '0 * * * *',
    description: 'Run at the start of every hour',
  },
  {
    label: 'Every 6 Hours',
    cron: '0 */6 * * *',
    description: 'Run every 6 hours',
  },
  {
    label: 'Daily at Midnight',
    cron: '0 0 * * *',
    description: 'Run once per day at 12:00 AM',
  },
  {
    label: 'Daily at 2 AM',
    cron: '0 2 * * *',
    description: 'Run once per day at 2:00 AM',
  },
  {
    label: 'Weekly (Sunday)',
    cron: '0 0 * * 0',
    description: 'Run every Sunday at midnight',
  },
  {
    label: 'Monthly',
    cron: '0 0 1 * *',
    description: 'Run on the 1st of every month',
  },
  {
    label: 'Custom',
    cron: '',
    description: 'Define your own cron expression',
  },
];

// ============================================================================
// ScheduleForm Component
// ============================================================================

export function ScheduleForm({
  schedule,
  onSubmit,
  onCancel,
  databases = [],
  loading = false,
}: ScheduleFormProps) {
  const [config, setConfig] = useState<CreateScheduleConfig>(() => {
    if (schedule) {
      return {
        name: schedule.name,
        type: schedule.type,
        database: schedule.database,
        schedule: schedule.schedule,
        retentionDays: schedule.retentionDays,
        isEnabled: schedule.isEnabled,
      };
    }
    return {
      name: '',
      type: 'full',
      schedule: '0 0 * * *',
      retentionDays: 30,
      compression: true,
      encrypted: false,
      isEnabled: true,
    };
  });

  const [selectedPreset, setSelectedPreset] = useState<string>('0 0 * * *');
  const [customCron, setCustomCron] = useState('');
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Update custom cron when schedule changes
  useEffect(() => {
    const preset = FREQUENCY_PRESETS.find((p) => p.cron === config.schedule);
    if (preset) {
      setSelectedPreset(preset.cron || 'custom');
      if (preset.cron === '') {
        setCustomCron(config.schedule);
      }
    } else {
      setSelectedPreset('custom');
      setCustomCron(config.schedule);
    }
  }, [config.schedule]);

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!config.name || config.name.trim() === '') {
      newErrors.name = 'Schedule name is required';
    }

    if (!config.schedule || config.schedule.trim() === '') {
      newErrors.schedule = 'Schedule is required';
    } else {
      // Basic cron validation (should have 5 parts)
      const parts = config.schedule.trim().split(/\s+/);
      if (parts.length !== 5) {
        newErrors.schedule = 'Invalid cron expression (expected 5 parts)';
      }
    }

    if (config.retentionDays < 1) {
      newErrors.retentionDays = 'Retention days must be at least 1';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) return;

    try {
      await onSubmit(config);
    } catch (error) {
      console.error('Failed to save schedule:', error);
    }
  };

  const handlePresetChange = (cron: string) => {
    setSelectedPreset(cron);
    if (cron !== 'custom' && cron !== '') {
      setConfig({ ...config, schedule: cron });
    }
  };

  const handleCustomCronChange = (value: string) => {
    setCustomCron(value);
    setConfig({ ...config, schedule: value });
  };

  const backupTypes: Array<{ value: BackupType; label: string }> = [
    { value: 'full', label: 'Full' },
    { value: 'incremental', label: 'Incremental' },
    { value: 'differential', label: 'Differential' },
    { value: 'logical', label: 'Logical' },
    { value: 'physical', label: 'Physical' },
  ];

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Schedule Name */}
      <div>
        <label className="block text-sm font-medium text-dark-200 mb-2">
          Schedule Name *
        </label>
        <input
          type="text"
          value={config.name}
          onChange={(e) => setConfig({ ...config, name: e.target.value })}
          placeholder="e.g., Daily Production Backup"
          className={clsx(
            'input w-full',
            errors.name && 'border-danger-500 focus:border-danger-500'
          )}
          disabled={loading}
        />
        {errors.name && (
          <p className="mt-1 text-sm text-danger-400">{errors.name}</p>
        )}
      </div>

      {/* Backup Type */}
      <div>
        <label className="block text-sm font-medium text-dark-200 mb-2">
          Backup Type *
        </label>
        <div className="grid grid-cols-2 md:grid-cols-5 gap-2">
          {backupTypes.map((type) => (
            <button
              key={type.value}
              type="button"
              onClick={() => setConfig({ ...config, type: type.value })}
              className={clsx(
                'px-4 py-2 rounded-lg border-2 text-sm font-medium transition-colors',
                config.type === type.value
                  ? 'border-rusty-500 bg-rusty-500/10 text-rusty-400'
                  : 'border-dark-600 text-dark-300 hover:border-dark-500'
              )}
              disabled={loading}
            >
              {type.label}
            </button>
          ))}
        </div>
      </div>

      {/* Target Database */}
      {databases.length > 0 && (
        <div>
          <label className="block text-sm font-medium text-dark-200 mb-2">
            Target Database (Optional)
          </label>
          <select
            value={config.database || ''}
            onChange={(e) =>
              setConfig({ ...config, database: e.target.value || undefined })
            }
            className="input w-full"
            disabled={loading}
          >
            <option value="">All Databases</option>
            {databases.map((db) => (
              <option key={db} value={db}>
                {db}
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Schedule Frequency */}
      <div>
        <label className="block text-sm font-medium text-dark-200 mb-3">
          <div className="flex items-center gap-2">
            <ClockIcon className="w-4 h-4" />
            Schedule Frequency *
          </div>
        </label>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-3">
          {FREQUENCY_PRESETS.map((preset) => (
            <button
              key={preset.label}
              type="button"
              onClick={() => handlePresetChange(preset.cron)}
              className={clsx(
                'p-3 rounded-lg border-2 text-left transition-colors',
                selectedPreset === (preset.cron || 'custom')
                  ? 'border-rusty-500 bg-rusty-500/10'
                  : 'border-dark-600 hover:border-dark-500 bg-dark-750'
              )}
              disabled={loading}
            >
              <div className="font-medium text-dark-100 text-sm">
                {preset.label}
              </div>
              <div className="text-xs text-dark-400 mt-1">
                {preset.description}
              </div>
            </button>
          ))}
        </div>

        {/* Custom Cron Expression */}
        {selectedPreset === 'custom' && (
          <div className="mt-3">
            <input
              type="text"
              value={customCron}
              onChange={(e) => handleCustomCronChange(e.target.value)}
              placeholder="0 0 * * * (minute hour day month weekday)"
              className={clsx(
                'input w-full font-mono text-sm',
                errors.schedule && 'border-danger-500 focus:border-danger-500'
              )}
              disabled={loading}
            />
            {errors.schedule && (
              <p className="mt-1 text-sm text-danger-400">{errors.schedule}</p>
            )}
            <div className="mt-2 p-3 bg-info-500/10 border border-info-500/30 rounded-lg">
              <div className="flex items-start gap-2">
                <InformationCircleIcon className="w-4 h-4 text-info-400 flex-shrink-0 mt-0.5" />
                <div className="text-xs text-info-300">
                  <p className="font-medium mb-1">Cron Expression Format:</p>
                  <p className="text-info-200">
                    minute (0-59) hour (0-23) day (1-31) month (1-12) weekday (0-6)
                  </p>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Cron Preview */}
        {config.schedule && !errors.schedule && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="mt-3 p-3 bg-dark-750 border border-dark-600 rounded-lg"
          >
            <div className="flex items-center gap-2 text-sm">
              <CalendarIcon className="w-4 h-4 text-rusty-400" />
              <span className="text-dark-300">
                Schedule: <span className="text-dark-100 font-medium">
                  {formatCronExpression(config.schedule)}
                </span>
              </span>
            </div>
          </motion.div>
        )}
      </div>

      {/* Retention Period */}
      <div>
        <label className="block text-sm font-medium text-dark-200 mb-2">
          Retention Period (days) *
        </label>
        <input
          type="number"
          value={config.retentionDays}
          onChange={(e) =>
            setConfig({ ...config, retentionDays: parseInt(e.target.value) || 30 })
          }
          min={1}
          max={3650}
          className={clsx(
            'input w-full',
            errors.retentionDays && 'border-danger-500 focus:border-danger-500'
          )}
          disabled={loading}
        />
        {errors.retentionDays && (
          <p className="mt-1 text-sm text-danger-400">{errors.retentionDays}</p>
        )}
      </div>

      {/* Options */}
      <div className="space-y-3">
        <label className="block text-sm font-medium text-dark-200">Options</label>

        {/* Compression */}
        <label className="flex items-center gap-3 p-3 rounded-lg bg-dark-750 border border-dark-600 cursor-pointer hover:border-dark-500 transition-colors">
          <input
            type="checkbox"
            checked={config.compression}
            onChange={(e) =>
              setConfig({ ...config, compression: e.target.checked })
            }
            className="checkbox"
            disabled={loading}
          />
          <div className="flex items-center gap-2 flex-1">
            <ArchiveBoxIcon className="w-5 h-5 text-blue-400" />
            <span className="text-sm text-dark-200">Enable Compression</span>
          </div>
        </label>

        {/* Encryption */}
        <label className="flex items-center gap-3 p-3 rounded-lg bg-dark-750 border border-dark-600 cursor-pointer hover:border-dark-500 transition-colors">
          <input
            type="checkbox"
            checked={config.encrypted}
            onChange={(e) =>
              setConfig({ ...config, encrypted: e.target.checked })
            }
            className="checkbox"
            disabled={loading}
          />
          <div className="flex items-center gap-2 flex-1">
            <ShieldCheckIcon className="w-5 h-5 text-success-400" />
            <span className="text-sm text-dark-200">Enable Encryption</span>
          </div>
        </label>

        {/* Enable Schedule */}
        <label className="flex items-center gap-3 p-3 rounded-lg bg-dark-750 border border-dark-600 cursor-pointer hover:border-dark-500 transition-colors">
          <input
            type="checkbox"
            checked={config.isEnabled}
            onChange={(e) =>
              setConfig({ ...config, isEnabled: e.target.checked })
            }
            className="checkbox"
            disabled={loading}
          />
          <div className="flex items-center gap-2 flex-1">
            <BellIcon className="w-5 h-5 text-warning-400" />
            <span className="text-sm text-dark-200">Enable Schedule</span>
          </div>
        </label>
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-3 pt-4 border-t border-dark-700">
        <button
          type="button"
          onClick={onCancel}
          className="btn-secondary"
          disabled={loading}
        >
          Cancel
        </button>
        <button type="submit" className="btn-primary" disabled={loading}>
          {loading ? (
            <>
              <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
              Saving...
            </>
          ) : (
            <>
              <CalendarIcon className="w-5 h-5" />
              {schedule ? 'Update Schedule' : 'Create Schedule'}
            </>
          )}
        </button>
      </div>
    </form>
  );
}
