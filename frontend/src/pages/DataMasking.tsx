// ============================================================================
// Data Masking Policies Page
// Manage data masking policies for sensitive data protection
// ============================================================================

import { useState } from 'react';
import { motion } from 'framer-motion';
import {
  EyeSlashIcon,
  PlusIcon,
  MagnifyingGlassIcon,
  FunnelIcon,
  PencilIcon,
  TrashIcon,
  PlayIcon,
  PauseIcon,
  CheckCircleIcon,
  XCircleIcon,
} from '@heroicons/react/24/outline';
import {
  useMaskingPolicies,
  useCreateMaskingPolicy,
  useUpdateMaskingPolicy,
  useDeleteMaskingPolicy,
  useToggleMaskingPolicy,
} from '../hooks/useSecurity';
import { MaskingPolicyForm } from '../components/security/MaskingPolicyForm';
import { LoadingScreen } from '../components/common/LoadingScreen';
import type { DataMaskingPolicy, MaskingType } from '@/types';
import clsx from 'clsx';

// ============================================================================
// Data Masking Component
// ============================================================================

export default function DataMasking() {
  const [searchTerm, setSearchTerm] = useState('');
  const [tableFilter, setTableFilter] = useState<string>('all');
  const [enabledFilter, setEnabledFilter] = useState<boolean | 'all'>('all');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [editingPolicy, setEditingPolicy] = useState<DataMaskingPolicy | null>(null);
  const [previewPolicy, setPreviewPolicy] = useState<DataMaskingPolicy | null>(null);

  const { data: policies, isLoading } = useMaskingPolicies();
  const createPolicy = useCreateMaskingPolicy();
  const updatePolicy = useUpdateMaskingPolicy();
  const deletePolicy = useDeleteMaskingPolicy();
  const togglePolicy = useToggleMaskingPolicy();

  // Get unique tables for filter
  const uniqueTables = Array.from(new Set(policies?.map((p) => p.table) || []));

  const filteredPolicies =
    policies?.filter((policy) => {
      const matchesSearch =
        !searchTerm ||
        policy.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        policy.table.toLowerCase().includes(searchTerm.toLowerCase()) ||
        policy.column.toLowerCase().includes(searchTerm.toLowerCase());
      const matchesTable = tableFilter === 'all' || policy.table === tableFilter;
      const matchesEnabled =
        enabledFilter === 'all' || policy.isEnabled === enabledFilter;
      return matchesSearch && matchesTable && matchesEnabled;
    }) || [];

  const activePolicies = policies?.filter((p) => p.isEnabled).length || 0;
  const maskedColumns = new Set(policies?.map((p) => `${p.table}.${p.column}`)).size;
  const affectedTables = new Set(policies?.map((p) => p.table)).size;

  const handleTogglePolicy = async (policyId: string, enabled: boolean) => {
    try {
      await togglePolicy.mutateAsync({ policyId, enabled });
    } catch (error) {
      console.error('Failed to toggle policy:', error);
    }
  };

  const handleDeletePolicy = async (policyId: string) => {
    if (
      confirm(
        'Are you sure you want to delete this masking policy? This action cannot be undone.'
      )
    ) {
      try {
        await deletePolicy.mutateAsync(policyId);
      } catch (error) {
        console.error('Failed to delete policy:', error);
      }
    }
  };

  if (isLoading) {
    return <LoadingScreen />;
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-dark-100 flex items-center gap-3">
            <EyeSlashIcon className="w-8 h-8 text-purple-500" />
            Data Masking Policies
          </h1>
          <p className="text-dark-400 mt-1">
            Configure data masking to protect sensitive information
          </p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary">
          <PlusIcon className="w-4 h-4" />
          Create Policy
        </button>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="card">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-purple-500/20 flex items-center justify-center">
              <EyeSlashIcon className="w-5 h-5 text-purple-400" />
            </div>
            <h3 className="text-sm font-medium text-dark-300">Active Policies</h3>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-3xl font-bold text-dark-100">{activePolicies}</span>
            <span className="text-sm text-dark-400">/ {policies?.length || 0} total</span>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
              <CheckCircleIcon className="w-5 h-5 text-blue-400" />
            </div>
            <h3 className="text-sm font-medium text-dark-300">Masked Columns</h3>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-3xl font-bold text-dark-100">{maskedColumns}</span>
            <span className="text-sm text-dark-400">columns</span>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-success-500/20 flex items-center justify-center">
              <CheckCircleIcon className="w-5 h-5 text-success-400" />
            </div>
            <h3 className="text-sm font-medium text-dark-300">Affected Tables</h3>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-3xl font-bold text-dark-100">{affectedTables}</span>
            <span className="text-sm text-dark-400">tables</span>
          </div>
        </div>

        <div className="card">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-warning-500/20 flex items-center justify-center">
              <PauseIcon className="w-5 h-5 text-warning-400" />
            </div>
            <h3 className="text-sm font-medium text-dark-300">Inactive</h3>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-3xl font-bold text-dark-100">
              {(policies?.length || 0) - activePolicies}
            </span>
            <span className="text-sm text-dark-400">disabled</span>
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
              placeholder="Search masking policies..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="input-field pl-10 w-full"
            />
          </div>

          {/* Table Filter */}
          <div className="flex items-center gap-2">
            <FunnelIcon className="w-5 h-5 text-dark-400" />
            <select
              value={tableFilter}
              onChange={(e) => setTableFilter(e.target.value)}
              className="input-field w-48"
            >
              <option value="all">All Tables</option>
              {uniqueTables.map((table) => (
                <option key={table} value={table}>
                  {table}
                </option>
              ))}
            </select>
          </div>

          {/* Status Filter */}
          <select
            value={enabledFilter === 'all' ? 'all' : enabledFilter ? 'enabled' : 'disabled'}
            onChange={(e) =>
              setEnabledFilter(
                e.target.value === 'all'
                  ? 'all'
                  : e.target.value === 'enabled'
                  ? true
                  : false
              )
            }
            className="input-field w-40"
          >
            <option value="all">All Status</option>
            <option value="enabled">Enabled</option>
            <option value="disabled">Disabled</option>
          </select>
        </div>
      </div>

      {/* Policies Table */}
      <div className="card">
        {filteredPolicies.length === 0 ? (
          <div className="text-center py-12">
            <EyeSlashIcon className="w-12 h-12 text-dark-400 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-dark-100 mb-2">No masking policies found</h3>
            <p className="text-dark-400 mb-4">
              {searchTerm || tableFilter !== 'all' || enabledFilter !== 'all'
                ? 'Try adjusting your filters'
                : 'Create your first masking policy to protect sensitive data'}
            </p>
            {!searchTerm && tableFilter === 'all' && enabledFilter === 'all' && (
              <button onClick={() => setShowCreateModal(true)} className="btn-primary">
                <PlusIcon className="w-4 h-4" />
                Create Policy
              </button>
            )}
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="table">
              <thead>
                <tr>
                  <th>Policy Name</th>
                  <th>Table</th>
                  <th>Column</th>
                  <th>Masking Type</th>
                  <th>Applied To</th>
                  <th>Status</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredPolicies.map((policy) => (
                  <tr key={policy.id}>
                    <td>
                      <div className="font-medium text-dark-100">{policy.name}</div>
                      {policy.description && (
                        <div className="text-xs text-dark-400 mt-1">{policy.description}</div>
                      )}
                    </td>
                    <td className="font-mono text-sm">{policy.table}</td>
                    <td className="font-mono text-sm font-medium">{policy.column}</td>
                    <td>
                      <span className="badge badge-primary">{getMaskingTypeLabel(policy.maskingType)}</span>
                    </td>
                    <td>
                      <div className="flex flex-wrap gap-1">
                        {policy.applyTo.slice(0, 3).map((role) => (
                          <span key={role} className="badge badge-secondary text-xs">
                            {role}
                          </span>
                        ))}
                        {policy.applyTo.length > 3 && (
                          <span className="badge badge-secondary text-xs">
                            +{policy.applyTo.length - 3}
                          </span>
                        )}
                      </div>
                    </td>
                    <td>
                      <button
                        onClick={() => handleTogglePolicy(policy.id, !policy.isEnabled)}
                        className={clsx(
                          'badge',
                          policy.isEnabled ? 'badge-success' : 'badge-secondary'
                        )}
                      >
                        {policy.isEnabled ? (
                          <>
                            <CheckCircleIcon className="w-3 h-3" />
                            Enabled
                          </>
                        ) : (
                          <>
                            <XCircleIcon className="w-3 h-3" />
                            Disabled
                          </>
                        )}
                      </button>
                    </td>
                    <td>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => setPreviewPolicy(policy)}
                          className="btn-ghost text-xs"
                          title="Preview masking"
                        >
                          <PlayIcon className="w-4 h-4" />
                        </button>
                        <button
                          onClick={() => setEditingPolicy(policy)}
                          className="btn-ghost text-xs"
                          title="Edit policy"
                        >
                          <PencilIcon className="w-4 h-4" />
                        </button>
                        <button
                          onClick={() => handleDeletePolicy(policy.id)}
                          className="btn-ghost text-xs text-danger-400 hover:text-danger-300"
                          title="Delete policy"
                        >
                          <TrashIcon className="w-4 h-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Create/Edit Policy Modal */}
      {(showCreateModal || editingPolicy) && (
        <MaskingPolicyForm
          policy={editingPolicy}
          onClose={() => {
            setShowCreateModal(false);
            setEditingPolicy(null);
          }}
          onCreate={createPolicy.mutateAsync}
          onUpdate={(policyId, request) =>
            updatePolicy.mutateAsync({ policyId, request })
          }
          isSubmitting={createPolicy.isPending || updatePolicy.isPending}
        />
      )}

      {/* Preview Modal */}
      {previewPolicy && (
        <MaskingPreviewModal
          policy={previewPolicy}
          onClose={() => setPreviewPolicy(null)}
        />
      )}
    </div>
  );
}

// ============================================================================
// Helper Functions
// ============================================================================

function getMaskingTypeLabel(type: MaskingType): string {
  const labels: Record<MaskingType, string> = {
    full: 'Full Mask',
    partial: 'Partial Mask',
    email: 'Email Mask',
    phone: 'Phone Mask',
    ssn: 'SSN Mask',
    credit_card: 'Credit Card Mask',
    custom: 'Custom',
    hash: 'Hash',
    null: 'Nullify',
  };
  return labels[type] || type;
}

// ============================================================================
// Masking Preview Modal Component
// ============================================================================

interface MaskingPreviewModalProps {
  policy: DataMaskingPolicy;
  onClose: () => void;
}

function MaskingPreviewModal({ policy, onClose }: MaskingPreviewModalProps) {
  const [sampleData] = useState([
    'john.doe@example.com',
    'jane.smith@company.org',
    '555-123-4567',
    '123-45-6789',
    '4532-1234-5678-9010',
  ]);

  // Mock masking preview (in real app, this would call the API)
  const getMaskedValue = (value: string) => {
    switch (policy.maskingType) {
      case 'email':
        return value.replace(/(.{2}).*@/, '$1***@');
      case 'phone':
        return value.replace(/\d(?=\d{4})/g, '*');
      case 'ssn':
        return value.replace(/\d(?=\d{4})/g, '*');
      case 'credit_card':
        return value.replace(/\d(?=\d{4})/g, '*');
      case 'full':
        return '*'.repeat(value.length);
      case 'partial':
        return value.slice(0, 2) + '*'.repeat(value.length - 4) + value.slice(-2);
      case 'hash':
        return `[HASH:${value.slice(0, 8)}...]`;
      case 'null':
        return 'NULL';
      default:
        return value;
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="card max-w-2xl w-full"
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-dark-100">Masking Preview</h2>
          <button onClick={onClose} className="btn-ghost">
            <XCircleIcon className="w-5 h-5" />
          </button>
        </div>

        <div className="mb-4 p-3 bg-dark-700/50 rounded-lg">
          <div className="grid grid-cols-2 gap-3 text-sm">
            <div>
              <span className="text-dark-400">Policy:</span>
              <span className="ml-2 font-medium text-dark-100">{policy.name}</span>
            </div>
            <div>
              <span className="text-dark-400">Type:</span>
              <span className="ml-2 font-medium text-dark-100">
                {getMaskingTypeLabel(policy.maskingType)}
              </span>
            </div>
            <div>
              <span className="text-dark-400">Table:</span>
              <span className="ml-2 font-mono text-dark-100">{policy.table}</span>
            </div>
            <div>
              <span className="text-dark-400">Column:</span>
              <span className="ml-2 font-mono text-dark-100">{policy.column}</span>
            </div>
          </div>
        </div>

        <div className="space-y-3">
          <h3 className="font-medium text-dark-100">Sample Data</h3>
          <div className="overflow-x-auto">
            <table className="table">
              <thead>
                <tr>
                  <th>Original Value</th>
                  <th>Masked Value</th>
                </tr>
              </thead>
              <tbody>
                {sampleData.map((value, idx) => (
                  <tr key={idx}>
                    <td className="font-mono text-sm">{value}</td>
                    <td className="font-mono text-sm text-purple-400">
                      {getMaskedValue(value)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        <div className="mt-6 flex justify-end">
          <button onClick={onClose} className="btn-primary">
            Close Preview
          </button>
        </div>
      </motion.div>
    </div>
  );
}
