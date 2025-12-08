import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  XMarkIcon,
  CloudArrowUpIcon,
  ShieldCheckIcon,
  ArchiveBoxIcon,
  CalendarIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import type { BackupType } from '../../types';
import type { CreateBackupConfig } from '../../services/backupService';

// ============================================================================
// Types
// ============================================================================

interface CreateBackupModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (config: CreateBackupConfig) => Promise<void>;
  databases?: string[];
  loading?: boolean;
}

// ============================================================================
// CreateBackupModal Component
// ============================================================================

export function CreateBackupModal({
  isOpen,
  onClose,
  onCreate,
  databases = [],
  loading = false,
}: CreateBackupModalProps) {
  const [config, setConfig] = useState<CreateBackupConfig>({
    type: 'full',
    compression: true,
    encrypted: false,
    retentionDays: 30,
  });

  const [errors, setErrors] = useState<Record<string, string>>({});

  // Reset form when modal closes
  useEffect(() => {
    if (!isOpen) {
      setConfig({
        type: 'full',
        compression: true,
        encrypted: false,
        retentionDays: 30,
      });
      setErrors({});
    }
  }, [isOpen]);

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!config.name || config.name.trim() === '') {
      newErrors.name = 'Backup name is required';
    }

    if (config.retentionDays !== undefined && config.retentionDays < 1) {
      newErrors.retentionDays = 'Retention days must be at least 1';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) return;

    try {
      await onCreate(config);
      onClose();
    } catch (error) {
      console.error('Failed to create backup:', error);
    }
  };

  const backupTypes: Array<{ value: BackupType; label: string; description: string }> = [
    {
      value: 'full',
      label: 'Full Backup',
      description: 'Complete backup of all data',
    },
    {
      value: 'incremental',
      label: 'Incremental',
      description: 'Only changes since last backup',
    },
    {
      value: 'differential',
      label: 'Differential',
      description: 'Changes since last full backup',
    },
    {
      value: 'logical',
      label: 'Logical',
      description: 'SQL dump of database structure and data',
    },
    {
      value: 'physical',
      label: 'Physical',
      description: 'Copy of database files',
    },
  ];

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50"
            onClick={onClose}
          />

          {/* Modal */}
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            <motion.div
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              className="card w-full max-w-2xl max-h-[90vh] overflow-y-auto"
              onClick={(e) => e.stopPropagation()}
            >
              {/* Header */}
              <div className="flex items-center justify-between p-6 border-b border-dark-700">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 bg-rusty-500/20 rounded-lg flex items-center justify-center">
                    <CloudArrowUpIcon className="w-6 h-6 text-rusty-400" />
                  </div>
                  <div>
                    <h2 className="text-xl font-semibold text-dark-100">Create Backup</h2>
                    <p className="text-sm text-dark-400 mt-0.5">
                      Configure and start a new backup
                    </p>
                  </div>
                </div>
                <button
                  onClick={onClose}
                  className="btn-ghost btn-sm"
                  disabled={loading}
                >
                  <XMarkIcon className="w-5 h-5" />
                </button>
              </div>

              {/* Form */}
              <form onSubmit={handleSubmit} className="p-6 space-y-6">
                {/* Backup Name */}
                <div>
                  <label className="block text-sm font-medium text-dark-200 mb-2">
                    Backup Name *
                  </label>
                  <input
                    type="text"
                    value={config.name || ''}
                    onChange={(e) =>
                      setConfig({ ...config, name: e.target.value })
                    }
                    placeholder="e.g., production-backup-2024-12-08"
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
                  <label className="block text-sm font-medium text-dark-200 mb-3">
                    Backup Type *
                  </label>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                    {backupTypes.map((type) => (
                      <button
                        key={type.value}
                        type="button"
                        onClick={() => setConfig({ ...config, type: type.value })}
                        className={clsx(
                          'p-4 rounded-lg border-2 text-left transition-colors',
                          config.type === type.value
                            ? 'border-rusty-500 bg-rusty-500/10'
                            : 'border-dark-600 hover:border-dark-500 bg-dark-750'
                        )}
                        disabled={loading}
                      >
                        <div className="font-medium text-dark-100">{type.label}</div>
                        <div className="text-sm text-dark-400 mt-1">
                          {type.description}
                        </div>
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
                        setConfig({
                          ...config,
                          database: e.target.value || undefined,
                        })
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
                    <p className="mt-1 text-sm text-dark-400">
                      Leave empty to backup all databases
                    </p>
                  </div>
                )}

                {/* Options */}
                <div className="space-y-4">
                  <label className="block text-sm font-medium text-dark-200">
                    Backup Options
                  </label>

                  {/* Compression */}
                  <label className="flex items-start gap-3 p-4 rounded-lg bg-dark-750 border border-dark-600 cursor-pointer hover:border-dark-500 transition-colors">
                    <input
                      type="checkbox"
                      checked={config.compression}
                      onChange={(e) =>
                        setConfig({ ...config, compression: e.target.checked })
                      }
                      className="mt-1 checkbox"
                      disabled={loading}
                    />
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <ArchiveBoxIcon className="w-5 h-5 text-blue-400" />
                        <span className="font-medium text-dark-100">
                          Enable Compression
                        </span>
                      </div>
                      <p className="text-sm text-dark-400 mt-1">
                        Compress backup data to save storage space
                      </p>
                    </div>
                  </label>

                  {/* Encryption */}
                  <label className="flex items-start gap-3 p-4 rounded-lg bg-dark-750 border border-dark-600 cursor-pointer hover:border-dark-500 transition-colors">
                    <input
                      type="checkbox"
                      checked={config.encrypted}
                      onChange={(e) =>
                        setConfig({ ...config, encrypted: e.target.checked })
                      }
                      className="mt-1 checkbox"
                      disabled={loading}
                    />
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <ShieldCheckIcon className="w-5 h-5 text-success-400" />
                        <span className="font-medium text-dark-100">
                          Enable Encryption
                        </span>
                      </div>
                      <p className="text-sm text-dark-400 mt-1">
                        Encrypt backup data for additional security
                      </p>
                    </div>
                  </label>
                </div>

                {/* Retention Days */}
                <div>
                  <label className="block text-sm font-medium text-dark-200 mb-2">
                    <div className="flex items-center gap-2">
                      <CalendarIcon className="w-4 h-4" />
                      Retention Period (days)
                    </div>
                  </label>
                  <input
                    type="number"
                    value={config.retentionDays || 30}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        retentionDays: parseInt(e.target.value) || 30,
                      })
                    }
                    min={1}
                    max={3650}
                    className={clsx(
                      'input w-full',
                      errors.retentionDays &&
                        'border-danger-500 focus:border-danger-500'
                    )}
                    disabled={loading}
                  />
                  {errors.retentionDays && (
                    <p className="mt-1 text-sm text-danger-400">
                      {errors.retentionDays}
                    </p>
                  )}
                  <p className="mt-1 text-sm text-dark-400">
                    Backup will be automatically deleted after this period
                  </p>
                </div>

                {/* Actions */}
                <div className="flex items-center justify-end gap-3 pt-4 border-t border-dark-700">
                  <button
                    type="button"
                    onClick={onClose}
                    className="btn-secondary"
                    disabled={loading}
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    className="btn-primary"
                    disabled={loading}
                  >
                    {loading ? (
                      <>
                        <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                        Creating...
                      </>
                    ) : (
                      <>
                        <CloudArrowUpIcon className="w-5 h-5" />
                        Create Backup
                      </>
                    )}
                  </button>
                </div>
              </form>
            </motion.div>
          </div>
        </>
      )}
    </AnimatePresence>
  );
}
