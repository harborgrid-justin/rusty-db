import { useState } from 'react';
import { motion } from 'framer-motion';
import {
  CloudArrowDownIcon,
  TrashIcon,
  CheckCircleIcon,
  XCircleIcon,
  ClockIcon,
  ArrowPathIcon,
  DocumentCheckIcon,
  EllipsisVerticalIcon,
  ShieldCheckIcon,
  ArchiveBoxIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import type { Backup, BackupStatus, BackupType } from '../../types';
import { formatBytes, formatDuration, formatDate } from '../../utils/format';

// ============================================================================
// Types
// ============================================================================

interface BackupListProps {
  backups: Backup[];
  onDownload?: (backup: Backup) => void;
  onDelete?: (backup: Backup) => void;
  onRestore?: (backup: Backup) => void;
  onVerify?: (backup: Backup) => void;
  loading?: boolean;
}

// ============================================================================
// Status Badge Component
// ============================================================================

function StatusBadge({ status }: { status: BackupStatus }) {
  const configs: Record<BackupStatus, { label: string; className: string; icon: React.ElementType }> = {
    completed: {
      label: 'Completed',
      className: 'badge-success',
      icon: CheckCircleIcon,
    },
    running: {
      label: 'Running',
      className: 'badge-info',
      icon: ArrowPathIcon,
    },
    pending: {
      label: 'Pending',
      className: 'badge-warning',
      icon: ClockIcon,
    },
    failed: {
      label: 'Failed',
      className: 'badge-danger',
      icon: XCircleIcon,
    },
    expired: {
      label: 'Expired',
      className: 'badge-secondary',
      icon: ClockIcon,
    },
    deleted: {
      label: 'Deleted',
      className: 'badge-secondary',
      icon: TrashIcon,
    },
  };

  const config = configs[status];
  const Icon = config.icon;

  return (
    <span className={clsx('badge', config.className)}>
      <Icon className="w-3.5 h-3.5" />
      {config.label}
    </span>
  );
}

// ============================================================================
// Type Badge Component
// ============================================================================

function TypeBadge({ type }: { type: BackupType }) {
  const configs: Record<BackupType, { label: string; className: string }> = {
    full: {
      label: 'Full',
      className: 'bg-rusty-500/20 text-rusty-400',
    },
    incremental: {
      label: 'Incremental',
      className: 'bg-blue-500/20 text-blue-400',
    },
    differential: {
      label: 'Differential',
      className: 'bg-purple-500/20 text-purple-400',
    },
    logical: {
      label: 'Logical',
      className: 'bg-green-500/20 text-green-400',
    },
    physical: {
      label: 'Physical',
      className: 'bg-orange-500/20 text-orange-400',
    },
  };

  const config = configs[type];

  return (
    <span className={clsx('badge', config.className)}>
      {config.label}
    </span>
  );
}

// ============================================================================
// Backup Row Component
// ============================================================================

interface BackupRowProps {
  backup: Backup;
  onDownload?: (backup: Backup) => void;
  onDelete?: (backup: Backup) => void;
  onRestore?: (backup: Backup) => void;
  onVerify?: (backup: Backup) => void;
}

function BackupRow({ backup, onDownload, onDelete, onRestore, onVerify }: BackupRowProps) {
  const [menuOpen, setMenuOpen] = useState(false);

  const canDownload = backup.status === 'completed';
  const canRestore = backup.status === 'completed';
  const canDelete = backup.status !== 'deleted';

  const compressionRatio = backup.compressedSize
    ? ((1 - backup.compressedSize / backup.size) * 100).toFixed(1)
    : null;

  return (
    <motion.tr
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="hover:bg-dark-700/50 transition-colors"
    >
      {/* Name & Database */}
      <td className="px-4 py-3">
        <div className="flex items-start gap-3">
          <div className="mt-1">
            <ArchiveBoxIcon className="w-5 h-5 text-dark-400" />
          </div>
          <div>
            <div className="font-medium text-dark-100">{backup.name}</div>
            {backup.database && (
              <div className="text-sm text-dark-400 mt-0.5">{backup.database}</div>
            )}
          </div>
        </div>
      </td>

      {/* Type */}
      <td className="px-4 py-3">
        <TypeBadge type={backup.type} />
      </td>

      {/* Status */}
      <td className="px-4 py-3">
        <StatusBadge status={backup.status} />
      </td>

      {/* Size */}
      <td className="px-4 py-3">
        <div>
          <div className="text-dark-100">{formatBytes(backup.size)}</div>
          {backup.compressedSize && compressionRatio && (
            <div className="text-xs text-dark-400 mt-0.5">
              {formatBytes(backup.compressedSize)} ({compressionRatio}% saved)
            </div>
          )}
        </div>
      </td>

      {/* Duration */}
      <td className="px-4 py-3 text-dark-200">
        {backup.duration ? formatDuration(backup.duration) : '-'}
      </td>

      {/* Date */}
      <td className="px-4 py-3">
        <div>
          <div className="text-dark-100">{formatDate(backup.startTime)}</div>
          {backup.endTime && (
            <div className="text-xs text-dark-400 mt-0.5">
              Ended {formatDate(backup.endTime)}
            </div>
          )}
        </div>
      </td>

      {/* Encrypted */}
      <td className="px-4 py-3 text-center">
        {backup.encrypted && (
          <ShieldCheckIcon className="w-5 h-5 text-success-400 inline-block" />
        )}
      </td>

      {/* Actions */}
      <td className="px-4 py-3">
        <div className="flex items-center justify-end gap-2">
          {canDownload && onDownload && (
            <button
              onClick={() => onDownload(backup)}
              className="btn-ghost btn-sm"
              title="Download backup"
            >
              <CloudArrowDownIcon className="w-4 h-4" />
            </button>
          )}

          {/* More actions menu */}
          <div className="relative">
            <button
              onClick={() => setMenuOpen(!menuOpen)}
              className="btn-ghost btn-sm"
            >
              <EllipsisVerticalIcon className="w-4 h-4" />
            </button>

            {menuOpen && (
              <>
                <div
                  className="fixed inset-0 z-10"
                  onClick={() => setMenuOpen(false)}
                />
                <div className="dropdown right-0 z-20">
                  {canRestore && onRestore && (
                    <button
                      onClick={() => {
                        onRestore(backup);
                        setMenuOpen(false);
                      }}
                      className="dropdown-item"
                    >
                      <ArrowPathIcon className="w-4 h-4" />
                      Restore
                    </button>
                  )}
                  {canDownload && onVerify && (
                    <button
                      onClick={() => {
                        onVerify(backup);
                        setMenuOpen(false);
                      }}
                      className="dropdown-item"
                    >
                      <DocumentCheckIcon className="w-4 h-4" />
                      Verify Integrity
                    </button>
                  )}
                  {canDelete && onDelete && (
                    <>
                      <div className="border-t border-dark-700 my-1" />
                      <button
                        onClick={() => {
                          onDelete(backup);
                          setMenuOpen(false);
                        }}
                        className="dropdown-item text-danger-400 hover:text-danger-300"
                      >
                        <TrashIcon className="w-4 h-4" />
                        Delete
                      </button>
                    </>
                  )}
                </div>
              </>
            )}
          </div>
        </div>
      </td>
    </motion.tr>
  );
}

// ============================================================================
// BackupList Component
// ============================================================================

export function BackupList({
  backups,
  onDownload,
  onDelete,
  onRestore,
  onVerify,
  loading = false,
}: BackupListProps) {
  if (loading) {
    return (
      <div className="card">
        <div className="p-8 text-center">
          <div className="inline-block w-8 h-8 border-4 border-dark-600 border-t-rusty-500 rounded-full animate-spin" />
          <p className="mt-4 text-dark-400">Loading backups...</p>
        </div>
      </div>
    );
  }

  if (backups.length === 0) {
    return (
      <div className="card">
        <div className="p-8 text-center">
          <ArchiveBoxIcon className="w-12 h-12 text-dark-600 mx-auto" />
          <h3 className="mt-4 text-lg font-medium text-dark-300">No backups found</h3>
          <p className="mt-2 text-dark-400">
            Create your first backup to get started
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="card overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead className="bg-dark-800 border-b border-dark-700">
            <tr>
              <th className="px-4 py-3 text-left text-xs font-medium text-dark-400 uppercase tracking-wider">
                Name
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-dark-400 uppercase tracking-wider">
                Type
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-dark-400 uppercase tracking-wider">
                Status
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-dark-400 uppercase tracking-wider">
                Size
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-dark-400 uppercase tracking-wider">
                Duration
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-dark-400 uppercase tracking-wider">
                Created
              </th>
              <th className="px-4 py-3 text-center text-xs font-medium text-dark-400 uppercase tracking-wider">
                Encrypted
              </th>
              <th className="px-4 py-3 text-right text-xs font-medium text-dark-400 uppercase tracking-wider">
                Actions
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-dark-700">
            {backups.map((backup) => (
              <BackupRow
                key={backup.id}
                backup={backup}
                onDownload={onDownload}
                onDelete={onDelete}
                onRestore={onRestore}
                onVerify={onVerify}
              />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
