import { useState } from 'react';
import { ResourceGroup } from '../../types';
import { Input } from '../common/Input';
import { Button } from '../common/Button';
import { MultiSelect } from '../common/Select';

// ============================================================================
// Resource Group Form Component
// Form for creating and editing resource groups
// ============================================================================

export interface ResourceGroupFormData {
  name: string;
  cpuLimit: number;
  memoryLimit: number;
  ioLimit?: number;
  maxConnections: number;
  maxQueries: number;
  queryTimeout: number;
  priority: number;
  members: string[];
  isEnabled: boolean;
}

export interface ResourceGroupFormProps {
  group?: ResourceGroup;
  availableMembers?: string[];
  onSubmit: (data: ResourceGroupFormData) => void | Promise<void>;
  onCancel: () => void;
  loading?: boolean;
}

export function ResourceGroupForm({
  group,
  availableMembers = [],
  onSubmit,
  onCancel,
  loading = false,
}: ResourceGroupFormProps) {
  const [formData, setFormData] = useState<ResourceGroupFormData>({
    name: group?.name || '',
    cpuLimit: group?.cpuLimit || 50,
    memoryLimit: group?.memoryLimit || 4 * 1024 * 1024 * 1024, // 4GB in bytes
    ioLimit: group?.ioLimit,
    maxConnections: group?.maxConnections || 10,
    maxQueries: group?.maxQueries || 100,
    queryTimeout: group?.queryTimeout || 30000, // 30 seconds
    priority: group?.priority || 5,
    members: group?.members || [],
    isEnabled: group?.isEnabled ?? true,
  });

  const [errors, setErrors] = useState<Partial<Record<keyof ResourceGroupFormData, string>>>({});

  const handleChange = (field: keyof ResourceGroupFormData, value: string | number | boolean | string[] | undefined) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    // Clear error when user starts typing
    if (errors[field]) {
      setErrors((prev) => ({ ...prev, [field]: undefined }));
    }
  };

  const validateForm = (): boolean => {
    const newErrors: Partial<Record<keyof ResourceGroupFormData, string>> = {};

    if (!formData.name.trim()) {
      newErrors.name = 'Name is required';
    }

    if (formData.cpuLimit < 1 || formData.cpuLimit > 100) {
      newErrors.cpuLimit = 'CPU limit must be between 1 and 100%';
    }

    if (formData.memoryLimit < 1024 * 1024 * 1024) {
      newErrors.memoryLimit = 'Memory limit must be at least 1 GB';
    }

    if (formData.maxConnections < 1) {
      newErrors.maxConnections = 'Max connections must be at least 1';
    }

    if (formData.maxQueries < 1) {
      newErrors.maxQueries = 'Max queries must be at least 1';
    }

    if (formData.queryTimeout < 1000) {
      newErrors.queryTimeout = 'Query timeout must be at least 1000ms';
    }

    if (formData.priority < 1 || formData.priority > 10) {
      newErrors.priority = 'Priority must be between 1 and 10';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validateForm()) {
      return;
    }

    await onSubmit(formData);
  };

  const formatMemoryGB = (bytes: number) => {
    return (bytes / (1024 * 1024 * 1024)).toFixed(2);
  };

  const parseMemoryGB = (gb: string) => {
    return parseFloat(gb) * 1024 * 1024 * 1024;
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Basic Information */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-dark-100">Basic Information</h3>

        <Input
          label="Group Name"
          value={formData.name}
          onChange={(e) => handleChange('name', e.target.value)}
          error={errors.name}
          placeholder="e.g., Production Team"
          required
          fullWidth
        />

        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="isEnabled"
            checked={formData.isEnabled}
            onChange={(e) => handleChange('isEnabled', e.target.checked)}
            className="rounded border-dark-600 bg-dark-700 text-rusty-500 focus:ring-rusty-500 focus:ring-offset-dark-800"
          />
          <label htmlFor="isEnabled" className="text-sm text-dark-300 cursor-pointer">
            Enable this resource group
          </label>
        </div>
      </div>

      {/* Resource Limits */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-dark-100">Resource Limits</h3>

        <div>
          <label className="block text-sm font-medium text-dark-300 mb-2">
            CPU Limit: {formData.cpuLimit}%
          </label>
          <input
            type="range"
            min="1"
            max="100"
            value={formData.cpuLimit}
            onChange={(e) => handleChange('cpuLimit', parseInt(e.target.value))}
            className="w-full h-2 bg-dark-700 rounded-lg appearance-none cursor-pointer accent-rusty-500"
          />
          {errors.cpuLimit && (
            <p className="mt-1 text-sm text-danger-500">{errors.cpuLimit}</p>
          )}
        </div>

        <Input
          label="Memory Limit (GB)"
          type="number"
          step="0.01"
          value={formatMemoryGB(formData.memoryLimit)}
          onChange={(e) => handleChange('memoryLimit', parseMemoryGB(e.target.value))}
          error={errors.memoryLimit}
          fullWidth
        />

        <Input
          label="I/O Limit (IOPS)"
          type="number"
          value={formData.ioLimit || ''}
          onChange={(e) =>
            handleChange('ioLimit', e.target.value ? parseInt(e.target.value) : undefined)
          }
          placeholder="Optional"
          helperText="Leave empty for unlimited I/O"
          fullWidth
        />
      </div>

      {/* Connection & Query Limits */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-dark-100">Connection & Query Settings</h3>

        <div className="grid grid-cols-2 gap-4">
          <Input
            label="Max Connections"
            type="number"
            min="1"
            value={formData.maxConnections}
            onChange={(e) => handleChange('maxConnections', parseInt(e.target.value))}
            error={errors.maxConnections}
            fullWidth
          />

          <Input
            label="Max Concurrent Queries"
            type="number"
            min="1"
            value={formData.maxQueries}
            onChange={(e) => handleChange('maxQueries', parseInt(e.target.value))}
            error={errors.maxQueries}
            fullWidth
          />
        </div>

        <Input
          label="Query Timeout (ms)"
          type="number"
          min="1000"
          step="1000"
          value={formData.queryTimeout}
          onChange={(e) => handleChange('queryTimeout', parseInt(e.target.value))}
          error={errors.queryTimeout}
          helperText="Queries exceeding this time will be terminated"
          fullWidth
        />
      </div>

      {/* Priority & Members */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-dark-100">Priority & Members</h3>

        <div>
          <label className="block text-sm font-medium text-dark-300 mb-2">
            Priority: {formData.priority}
          </label>
          <input
            type="range"
            min="1"
            max="10"
            value={formData.priority}
            onChange={(e) => handleChange('priority', parseInt(e.target.value))}
            className="w-full h-2 bg-dark-700 rounded-lg appearance-none cursor-pointer accent-rusty-500"
          />
          <div className="flex justify-between text-xs text-dark-400 mt-1">
            <span>Low (1)</span>
            <span>Medium (5)</span>
            <span>High (10)</span>
          </div>
          {errors.priority && (
            <p className="mt-1 text-sm text-danger-500">{errors.priority}</p>
          )}
        </div>

        <MultiSelect
          label="Group Members"
          value={formData.members}
          onChange={(value) => handleChange('members', value)}
          options={availableMembers.map((member) => ({ value: member, label: member }))}
          helperText="Select users or roles to assign to this group"
          fullWidth
        />
      </div>

      {/* Actions */}
      <div className="flex justify-end gap-3 pt-4 border-t border-dark-700">
        <Button variant="ghost" onClick={onCancel} disabled={loading}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" loading={loading}>
          {group ? 'Update' : 'Create'} Resource Group
        </Button>
      </div>
    </form>
  );
}
