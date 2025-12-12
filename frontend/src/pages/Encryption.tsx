// ============================================================================
// Encryption Management Page
// Manage encryption keys, TDE, and encrypted tables
// ============================================================================

import { useState } from 'react';
import { motion } from 'framer-motion';
import {
  LockClosedIcon,
  PlusIcon,
  MagnifyingGlassIcon,
  FunnelIcon,
  ArrowPathIcon,
  ExclamationTriangleIcon,
} from '@heroicons/react/24/outline';
import {
  useEncryptionKeys,
  useEncryptedTables,
  useCreateEncryptionKey,
  useDeleteKey,
} from '../hooks/useSecurity';
import { EncryptionKeyCard } from '../components/security/EncryptionKeyCard';
import { KeyRotationWizard } from '../components/security/KeyRotationWizard';
import { LoadingScreen } from '../components/common/LoadingScreen';
import type { EncryptionAlgorithm, KeyType } from '@/types';
import clsx from 'clsx';

// ============================================================================
// Encryption Management Component
// ============================================================================

export default function Encryption() {
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [typeFilter, setTypeFilter] = useState<KeyType | 'all'>('all');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showRotationWizard, setShowRotationWizard] = useState(false);
  const [selectedKeyId, setSelectedKeyId] = useState<string | null>(null);

  const { data: keys, isLoading: keysLoading } = useEncryptionKeys(
    typeFilter !== 'all' ? { keyType: typeFilter } : undefined
  );
  const { data: encryptedTables, isLoading: tablesLoading } = useEncryptedTables();
  const createKey = useCreateEncryptionKey();
  const deleteKey = useDeleteKey();

  const filteredKeys =
    keys?.filter((key) => {
      const matchesSearch =
        !searchTerm ||
        key.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        key.id.toLowerCase().includes(searchTerm.toLowerCase());
      const matchesStatus = statusFilter === 'all' || key.status === statusFilter;
      return matchesSearch && matchesStatus;
    }) || [];

  const activeKeys = keys?.filter((k) => k.status === 'active').length || 0;
  const expiringKeys =
    keys?.filter((k) => {
      if (!k.expiresAt) return false;
      const daysUntilExpiry =
        (new Date(k.expiresAt).getTime() - Date.now()) / (1000 * 60 * 60 * 24);
      return daysUntilExpiry <= 30 && daysUntilExpiry > 0;
    }).length || 0;

  const handleRotateKey = (keyId: string) => {
    setSelectedKeyId(keyId);
    setShowRotationWizard(true);
  };

  const handleDeleteKey = async (keyId: string) => {
    if (confirm('Are you sure you want to delete this encryption key? This action cannot be undone.')) {
      try {
        await deleteKey.mutateAsync(keyId);
      } catch (error) {
        console.error('Failed to delete key:', error);
      }
    }
  };

  if (keysLoading) {
    return <LoadingScreen />;
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-dark-100 flex items-center gap-3">
            <LockClosedIcon className="w-8 h-8 text-rusty-500" />
            Encryption Management
          </h1>
          <p className="text-dark-400 mt-1">
            Manage encryption keys and transparent data encryption
          </p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary">
          <PlusIcon className="w-4 h-4" />
          Create Encryption Key
        </button>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="card">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-rusty-500/20 flex items-center justify-center">
              <LockClosedIcon className="w-5 h-5 text-rusty-400" />
            </div>
            <h3 className="text-sm font-medium text-dark-300">Active Keys</h3>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-3xl font-bold text-dark-100">{activeKeys}</span>
            <span className="text-sm text-dark-400">/ {keys?.length || 0} total</span>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
              <LockClosedIcon className="w-5 h-5 text-blue-400" />
            </div>
            <h3 className="text-sm font-medium text-dark-300">Encrypted Tables</h3>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-3xl font-bold text-dark-100">
              {encryptedTables?.length || 0}
            </span>
            <span className="text-sm text-dark-400">tables</span>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-warning-500/20 flex items-center justify-center">
              <ExclamationTriangleIcon className="w-5 h-5 text-warning-400" />
            </div>
            <h3 className="text-sm font-medium text-dark-300">Expiring Soon</h3>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-3xl font-bold text-dark-100">{expiringKeys}</span>
            <span className="text-sm text-dark-400">keys</span>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-success-500/20 flex items-center justify-center">
              <ArrowPathIcon className="w-5 h-5 text-success-400" />
            </div>
            <h3 className="text-sm font-medium text-dark-300">Recent Rotations</h3>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-3xl font-bold text-dark-100">
              {keys?.filter((k) => k.rotatedAt).length || 0}
            </span>
            <span className="text-sm text-dark-400">rotated</span>
          </div>
        </div>
      </div>

      {/* Filters and Search */}
      <div className="card">
        <div className="flex flex-col sm:flex-row gap-4">
          {/* Search */}
          <div className="flex-1 relative">
            <MagnifyingGlassIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-dark-400" />
            <input
              type="text"
              placeholder="Search encryption keys..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="input-field pl-10 w-full"
            />
          </div>

          {/* Status Filter */}
          <div className="flex items-center gap-2">
            <FunnelIcon className="w-5 h-5 text-dark-400" />
            <select
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value)}
              className="input-field w-40"
            >
              <option value="all">All Status</option>
              <option value="active">Active</option>
              <option value="inactive">Inactive</option>
              <option value="expired">Expired</option>
              <option value="pending_rotation">Pending Rotation</option>
            </select>
          </div>

          {/* Type Filter */}
          <select
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value as KeyType | 'all')}
            className="input-field w-40"
          >
            <option value="all">All Types</option>
            <option value="master">Master</option>
            <option value="data">Data</option>
            <option value="backup">Backup</option>
            <option value="transport">Transport</option>
          </select>
        </div>
      </div>

      {/* Encryption Keys Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {filteredKeys.length === 0 ? (
          <div className="col-span-full card text-center py-12">
            <LockClosedIcon className="w-12 h-12 text-dark-400 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-dark-100 mb-2">No encryption keys found</h3>
            <p className="text-dark-400 mb-4">
              {searchTerm || statusFilter !== 'all' || typeFilter !== 'all'
                ? 'Try adjusting your filters'
                : 'Create your first encryption key to get started'}
            </p>
            {!searchTerm && statusFilter === 'all' && typeFilter === 'all' && (
              <button onClick={() => setShowCreateModal(true)} className="btn-primary">
                <PlusIcon className="w-4 h-4" />
                Create Encryption Key
              </button>
            )}
          </div>
        ) : (
          filteredKeys.map((key) => (
            <EncryptionKeyCard
              key={key.id}
              encryptionKey={key}
              onRotate={handleRotateKey}
              onDelete={handleDeleteKey}
            />
          ))
        )}
      </div>

      {/* Encrypted Tables Section */}
      {encryptedTables && encryptedTables.length > 0 && (
        <div className="card">
          <h2 className="text-lg font-semibold text-dark-100 mb-4">Encrypted Tables</h2>
          <div className="overflow-x-auto">
            <table className="table">
              <thead>
                <tr>
                  <th>Schema</th>
                  <th>Table</th>
                  <th>Encrypted Columns</th>
                  <th>Encryption Key</th>
                  <th>Encrypted At</th>
                </tr>
              </thead>
              <tbody>
                {encryptedTables.map((table, idx) => (
                  <tr key={`${table.schema}.${table.table}`}>
                    <td className="font-mono text-sm">{table.schema}</td>
                    <td className="font-mono text-sm font-medium">{table.table}</td>
                    <td>
                      <div className="flex flex-wrap gap-1">
                        {table.encryptedColumns.map((col) => (
                          <span key={col} className="badge badge-secondary text-xs">
                            {col}
                          </span>
                        ))}
                      </div>
                    </td>
                    <td className="font-mono text-xs text-dark-400">{table.keyId}</td>
                    <td className="text-sm text-dark-400">
                      {new Date(table.encryptedAt).toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Create Key Modal */}
      {showCreateModal && (
        <CreateKeyModal
          onClose={() => setShowCreateModal(false)}
          onCreate={createKey.mutateAsync}
          isCreating={createKey.isPending}
        />
      )}

      {/* Key Rotation Wizard */}
      {showRotationWizard && selectedKeyId && (
        <KeyRotationWizard
          keyId={selectedKeyId}
          onClose={() => {
            setShowRotationWizard(false);
            setSelectedKeyId(null);
          }}
        />
      )}
    </div>
  );
}

// ============================================================================
// Create Key Modal Component
// ============================================================================

interface CreateKeyModalProps {
  onClose: () => void;
  onCreate: (request: any) => Promise<any>;
  isCreating: boolean;
}

function CreateKeyModal({ onClose, onCreate, isCreating }: CreateKeyModalProps) {
  const [name, setName] = useState('');
  const [algorithm, setAlgorithm] = useState<EncryptionAlgorithm>('AES256GCM');
  const [keyType, setKeyType] = useState<KeyType>('data');
  const [expiresInDays, setExpiresInDays] = useState<number>(365);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await onCreate({
        name,
        algorithm,
        keyType,
        expiresInDays,
      });
      onClose();
    } catch (error) {
      console.error('Failed to create key:', error);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="card max-w-lg w-full"
      >
        <h2 className="text-xl font-bold text-dark-100 mb-4">Create Encryption Key</h2>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="label">Key Name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input-field w-full"
              placeholder="e.g., production-master-key"
              required
            />
          </div>

          <div>
            <label className="label">Algorithm</label>
            <select
              value={algorithm}
              onChange={(e) => setAlgorithm(e.target.value as EncryptionAlgorithm)}
              className="input-field w-full"
            >
              <option value="AES256GCM">AES-256-GCM (Recommended)</option>
              <option value="ChaCha20Poly1305">ChaCha20-Poly1305</option>
              <option value="RSA4096">RSA-4096</option>
            </select>
          </div>

          <div>
            <label className="label">Key Type</label>
            <select
              value={keyType}
              onChange={(e) => setKeyType(e.target.value as KeyType)}
              className="input-field w-full"
            >
              <option value="master">Master Key</option>
              <option value="data">Data Encryption Key</option>
              <option value="backup">Backup Key</option>
              <option value="transport">Transport Key</option>
            </select>
          </div>

          <div>
            <label className="label">Expires In (Days)</label>
            <input
              type="number"
              value={expiresInDays}
              onChange={(e) => setExpiresInDays(parseInt(e.target.value))}
              className="input-field w-full"
              min="1"
              max="3650"
            />
            <p className="text-xs text-dark-400 mt-1">
              Recommended: 365 days for data keys, 730 days for master keys
            </p>
          </div>

          <div className="flex gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              disabled={isCreating}
              className="btn-secondary flex-1"
            >
              Cancel
            </button>
            <button type="submit" disabled={isCreating} className="btn-primary flex-1">
              {isCreating ? 'Creating...' : 'Create Key'}
            </button>
          </div>
        </form>
      </motion.div>
    </div>
  );
}
