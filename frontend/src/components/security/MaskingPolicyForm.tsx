// ============================================================================
// Masking Policy Form Component
// Create or edit data masking policies
// ============================================================================

import { useState } from 'react';
import { motion } from 'framer-motion';
import {
  XMarkIcon,
  EyeSlashIcon,
  PlusIcon,
  TrashIcon,
  InformationCircleIcon,
} from '@heroicons/react/24/outline';
import type { DataMaskingPolicy, MaskingType } from '../../types';
import type {
  CreateMaskingPolicyRequest,
  UpdateMaskingPolicyRequest,
} from '../../services/securityService';

// ============================================================================
// Component Props
// ============================================================================

interface MaskingPolicyFormProps {
  policy?: DataMaskingPolicy | null;
  onClose: () => void;
  onCreate: (request: CreateMaskingPolicyRequest) => Promise<void>;
  onUpdate: (policyId: string, request: UpdateMaskingPolicyRequest) => Promise<void>;
  isSubmitting: boolean;
}

// ============================================================================
// Masking Policy Form Component
// ============================================================================

export function MaskingPolicyForm({
  policy,
  onClose,
  onCreate,
  onUpdate,
  isSubmitting,
}: MaskingPolicyFormProps) {
  const isEditMode = !!policy;

  const [name, setName] = useState(policy?.name || '');
  const [description, setDescription] = useState(policy?.description || '');
  const [table, setTable] = useState(policy?.table || '');
  const [column, setColumn] = useState(policy?.column || '');
  const [maskingType, setMaskingType] = useState<MaskingType>(
    policy?.maskingType || 'partial'
  );
  const [maskingFunction, setMaskingFunction] = useState(policy?.maskingFunction || '');
  const [applyTo, setApplyTo] = useState<string[]>(policy?.applyTo || []);
  const [newRole, setNewRole] = useState('');
  const [error, setError] = useState<string | null>(null);

  const maskingTypes: Array<{ value: MaskingType; label: string; description: string }> = [
    {
      value: 'full',
      label: 'Full Mask',
      description: 'Replace all characters with asterisks',
    },
    {
      value: 'partial',
      label: 'Partial Mask',
      description: 'Show first and last 2 characters, mask the rest',
    },
    {
      value: 'email',
      label: 'Email Mask',
      description: 'Mask email username but keep domain',
    },
    {
      value: 'phone',
      label: 'Phone Mask',
      description: 'Mask all digits except last 4',
    },
    {
      value: 'ssn',
      label: 'SSN Mask',
      description: 'Mask all digits except last 4 (XXX-XX-1234)',
    },
    {
      value: 'credit_card',
      label: 'Credit Card Mask',
      description: 'Mask all digits except last 4',
    },
    {
      value: 'hash',
      label: 'Hash',
      description: 'Replace with cryptographic hash',
    },
    {
      value: 'null',
      label: 'Nullify',
      description: 'Replace with NULL value',
    },
    {
      value: 'custom',
      label: 'Custom Function',
      description: 'Use a custom masking function',
    },
  ];

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!name || !table || !column || !maskingType) {
      setError('Please fill in all required fields');
      return;
    }

    if (applyTo.length === 0) {
      setError('Please specify at least one role to apply this policy to');
      return;
    }

    if (maskingType === 'custom' && !maskingFunction) {
      setError('Please provide a custom masking function');
      return;
    }

    try {
      if (isEditMode && policy) {
        await onUpdate(policy.id, {
          name,
          description,
          maskingType,
          maskingFunction: maskingType === 'custom' ? maskingFunction : undefined,
          applyTo,
        });
      } else {
        await onCreate({
          name,
          description,
          table,
          column,
          maskingType,
          maskingFunction: maskingType === 'custom' ? maskingFunction : undefined,
          applyTo,
        });
      }
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save policy');
    }
  };

  const handleAddRole = () => {
    if (newRole && !applyTo.includes(newRole)) {
      setApplyTo([...applyTo, newRole]);
      setNewRole('');
    }
  };

  const handleRemoveRole = (role: string) => {
    setApplyTo(applyTo.filter((r) => r !== role));
  };

  const selectedMaskingType = maskingTypes.find((t) => t.value === maskingType);

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="card max-w-2xl w-full max-h-[90vh] overflow-y-auto"
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-6 sticky top-0 bg-dark-800 pb-4 border-b border-dark-700">
          <h2 className="text-xl font-bold text-dark-100 flex items-center gap-3">
            <EyeSlashIcon className="w-6 h-6 text-purple-500" />
            {isEditMode ? 'Edit Masking Policy' : 'Create Masking Policy'}
          </h2>
          <button onClick={onClose} className="btn-ghost">
            <XMarkIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Error Message */}
        {error && (
          <div className="mb-4 p-3 rounded-lg bg-danger-500/10 border border-danger-500/30">
            <p className="text-sm text-danger-400">{error}</p>
          </div>
        )}

        {/* Form */}
        <form onSubmit={handleSubmit} className="space-y-6">
          {/* Policy Name */}
          <div>
            <label className="label">
              Policy Name <span className="text-danger-400">*</span>
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input-field w-full"
              placeholder="e.g., mask-customer-email"
              required
            />
          </div>

          {/* Description */}
          <div>
            <label className="label">Description</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="input-field w-full resize-none"
              rows={2}
              placeholder="Optional description of this policy"
            />
          </div>

          {/* Table and Column (disabled in edit mode) */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="label">
                Table <span className="text-danger-400">*</span>
              </label>
              <input
                type="text"
                value={table}
                onChange={(e) => setTable(e.target.value)}
                className="input-field w-full"
                placeholder="e.g., customers"
                disabled={isEditMode}
                required
              />
            </div>

            <div>
              <label className="label">
                Column <span className="text-danger-400">*</span>
              </label>
              <input
                type="text"
                value={column}
                onChange={(e) => setColumn(e.target.value)}
                className="input-field w-full"
                placeholder="e.g., email"
                disabled={isEditMode}
                required
              />
            </div>
          </div>

          {/* Masking Type */}
          <div>
            <label className="label">
              Masking Type <span className="text-danger-400">*</span>
            </label>
            <select
              value={maskingType}
              onChange={(e) => setMaskingType(e.target.value as MaskingType)}
              className="input-field w-full"
              required
            >
              {maskingTypes.map((type) => (
                <option key={type.value} value={type.value}>
                  {type.label}
                </option>
              ))}
            </select>
            {selectedMaskingType && (
              <div className="mt-2 flex items-start gap-2 text-sm text-dark-400">
                <InformationCircleIcon className="w-4 h-4 mt-0.5 flex-shrink-0" />
                <p>{selectedMaskingType.description}</p>
              </div>
            )}
          </div>

          {/* Custom Masking Function */}
          {maskingType === 'custom' && (
            <div>
              <label className="label">
                Custom Masking Function <span className="text-danger-400">*</span>
              </label>
              <textarea
                value={maskingFunction}
                onChange={(e) => setMaskingFunction(e.target.value)}
                className="input-field w-full font-mono text-sm resize-none"
                rows={4}
                placeholder="CASE WHEN length($1) > 4 THEN concat(left($1, 2), '***', right($1, 2)) ELSE '****' END"
                required
              />
              <p className="text-xs text-dark-400 mt-1">
                Use $1 to reference the column value in your function
              </p>
            </div>
          )}

          {/* Apply To Roles */}
          <div>
            <label className="label">
              Apply To Roles <span className="text-danger-400">*</span>
            </label>
            <div className="flex gap-2 mb-3">
              <input
                type="text"
                value={newRole}
                onChange={(e) => setNewRole(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && (e.preventDefault(), handleAddRole())}
                className="input-field flex-1"
                placeholder="Enter role name"
              />
              <button
                type="button"
                onClick={handleAddRole}
                className="btn-secondary"
              >
                <PlusIcon className="w-4 h-4" />
                Add
              </button>
            </div>

            {applyTo.length > 0 ? (
              <div className="space-y-2">
                {applyTo.map((role) => (
                  <div
                    key={role}
                    className="flex items-center justify-between p-3 rounded-lg bg-dark-700/50"
                  >
                    <span className="text-sm text-dark-100 font-medium">{role}</span>
                    <button
                      type="button"
                      onClick={() => handleRemoveRole(role)}
                      className="btn-ghost text-xs text-danger-400 hover:text-danger-300"
                    >
                      <TrashIcon className="w-4 h-4" />
                    </button>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-4 rounded-lg bg-dark-700/30 border border-dark-700 text-center">
                <p className="text-sm text-dark-400">No roles added yet</p>
              </div>
            )}
          </div>

          {/* Preview Section */}
          <div className="p-4 rounded-lg bg-dark-700/30 border border-dark-700">
            <h3 className="text-sm font-medium text-dark-100 mb-3">Policy Preview</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-dark-400">Target:</span>
                <span className="font-mono text-dark-100">
                  {table || '...'}.{column || '...'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-dark-400">Masking:</span>
                <span className="text-dark-100">
                  {selectedMaskingType?.label || 'Not selected'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-dark-400">Applied to:</span>
                <span className="text-dark-100">
                  {applyTo.length > 0 ? `${applyTo.length} role(s)` : 'None'}
                </span>
              </div>
            </div>
          </div>

          {/* Actions */}
          <div className="flex gap-3 pt-4 border-t border-dark-700">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="btn-secondary flex-1"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="btn-primary flex-1"
            >
              {isSubmitting
                ? 'Saving...'
                : isEditMode
                ? 'Update Policy'
                : 'Create Policy'}
            </button>
          </div>
        </form>
      </motion.div>
    </div>
  );
}
