// ============================================================================
// Encryption Key Card Component
// Displays encryption key details with actions
// ============================================================================

import { motion } from 'framer-motion';
import {
  KeyIcon,
  ArrowPathIcon,
  TrashIcon,
  ClockIcon,
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  InformationCircleIcon,
} from '@heroicons/react/24/outline';
import type { EncryptionKey } from '../../types';
import clsx from 'clsx';

// ============================================================================
// Component Props
// ============================================================================

interface EncryptionKeyCardProps {
  encryptionKey: EncryptionKey;
  onRotate: (keyId: string) => void;
  onDelete: (keyId: string) => void;
}

// ============================================================================
// Encryption Key Card Component
// ============================================================================

export function EncryptionKeyCard({
  encryptionKey,
  onRotate,
  onDelete,
}: EncryptionKeyCardProps) {
  const { id, name, algorithm, keyType, status, createdAt, expiresAt, rotatedAt, version } =
    encryptionKey;

  // Calculate days until expiration
  const daysUntilExpiry = expiresAt
    ? Math.ceil((new Date(expiresAt).getTime() - Date.now()) / (1000 * 60 * 60 * 24))
    : null;

  const isExpiringSoon = daysUntilExpiry !== null && daysUntilExpiry <= 30 && daysUntilExpiry > 0;
  const isExpired = daysUntilExpiry !== null && daysUntilExpiry <= 0;

  // Get status badge config
  const getStatusBadge = () => {
    switch (status) {
      case 'active':
        return {
          icon: CheckCircleIcon,
          className: 'badge-success',
          label: 'Active',
        };
      case 'inactive':
        return {
          icon: XCircleIcon,
          className: 'badge-secondary',
          label: 'Inactive',
        };
      case 'expired':
        return {
          icon: XCircleIcon,
          className: 'badge-danger',
          label: 'Expired',
        };
      case 'compromised':
        return {
          icon: ExclamationTriangleIcon,
          className: 'badge-danger',
          label: 'Compromised',
        };
      case 'pending_rotation':
        return {
          icon: ArrowPathIcon,
          className: 'badge-warning',
          label: 'Pending Rotation',
        };
      default:
        return {
          icon: InformationCircleIcon,
          className: 'badge-secondary',
          label: status,
        };
    }
  };

  const statusBadge = getStatusBadge();
  const StatusIcon = statusBadge.icon;

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="card group hover:border-rusty-500/30 transition-colors"
    >
      {/* Header */}
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-rusty-500/20 flex items-center justify-center">
              <KeyIcon className="w-5 h-5 text-rusty-400" />
            </div>
            <div className="flex-1">
              <h3 className="font-semibold text-dark-100 truncate">{name}</h3>
              <p className="text-xs text-dark-400 font-mono">{id}</p>
            </div>
          </div>
        </div>
        <span className={clsx('badge', statusBadge.className)}>
          <StatusIcon className="w-3 h-3" />
          {statusBadge.label}
        </span>
      </div>

      {/* Key Details */}
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <p className="text-xs text-dark-400 mb-1">Algorithm</p>
          <p className="text-sm font-medium text-dark-100">{algorithm}</p>
        </div>
        <div>
          <p className="text-xs text-dark-400 mb-1">Key Type</p>
          <p className="text-sm font-medium text-dark-100 capitalize">
            {keyType.replace('_', ' ')}
          </p>
        </div>
        <div>
          <p className="text-xs text-dark-400 mb-1">Version</p>
          <p className="text-sm font-medium text-dark-100">v{version}</p>
        </div>
        <div>
          <p className="text-xs text-dark-400 mb-1">Created</p>
          <p className="text-sm font-medium text-dark-100">
            {new Date(createdAt).toLocaleDateString()}
          </p>
        </div>
      </div>

      {/* Expiration Warning */}
      {(isExpiringSoon || isExpired) && (
        <div
          className={clsx(
            'flex items-center gap-2 p-3 rounded-lg mb-4',
            isExpired
              ? 'bg-danger-500/10 border border-danger-500/30'
              : 'bg-warning-500/10 border border-warning-500/30'
          )}
        >
          <ExclamationTriangleIcon
            className={clsx('w-5 h-5', isExpired ? 'text-danger-400' : 'text-warning-400')}
          />
          <div className="flex-1">
            <p
              className={clsx(
                'text-sm font-medium',
                isExpired ? 'text-danger-400' : 'text-warning-400'
              )}
            >
              {isExpired
                ? 'Key Expired'
                : `Expires in ${daysUntilExpiry} day${daysUntilExpiry === 1 ? '' : 's'}`}
            </p>
            {expiresAt && (
              <p className="text-xs text-dark-400">
                {new Date(expiresAt).toLocaleDateString()}
              </p>
            )}
          </div>
        </div>
      )}

      {/* Rotation History */}
      {rotatedAt && (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-dark-700/50 mb-4">
          <ClockIcon className="w-5 h-5 text-success-400" />
          <div className="flex-1">
            <p className="text-sm font-medium text-dark-100">Last Rotated</p>
            <p className="text-xs text-dark-400">{new Date(rotatedAt).toLocaleString()}</p>
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-2 pt-4 border-t border-dark-700">
        <button
          onClick={() => onRotate(id)}
          disabled={status === 'inactive' || status === 'compromised'}
          className="btn-secondary flex-1 text-sm"
          title="Rotate encryption key"
        >
          <ArrowPathIcon className="w-4 h-4" />
          Rotate Key
        </button>
        <button
          onClick={() => onDelete(id)}
          disabled={status === 'active'}
          className="btn-ghost text-sm text-danger-400 hover:text-danger-300"
          title="Delete encryption key"
        >
          <TrashIcon className="w-4 h-4" />
        </button>
      </div>

      {/* Additional Info */}
      {status === 'active' && (
        <div className="mt-3 text-xs text-dark-500">
          Active keys cannot be deleted. Deactivate first.
        </div>
      )}
    </motion.div>
  );
}
