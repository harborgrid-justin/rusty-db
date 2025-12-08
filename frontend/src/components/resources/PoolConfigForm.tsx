import { useState } from 'react';
import { Input } from '../common/Input';
import { Button } from '../common/Button';

// ============================================================================
// Pool Config Form Component
// Form for configuring connection pools
// ============================================================================

export interface PoolConfig {
  poolId: string;
  minConnections: number;
  maxConnections: number;
  connectionTimeout: number;
  idleTimeout: number;
  validationInterval: number;
}

export interface PoolConfigFormProps {
  config?: PoolConfig;
  onSubmit: (config: PoolConfig) => void | Promise<void>;
  onCancel: () => void;
  loading?: boolean;
}

export function PoolConfigForm({
  config,
  onSubmit,
  onCancel,
  loading = false,
}: PoolConfigFormProps) {
  const [formData, setFormData] = useState<PoolConfig>({
    poolId: config?.poolId || '',
    minConnections: config?.minConnections || 5,
    maxConnections: config?.maxConnections || 20,
    connectionTimeout: config?.connectionTimeout || 30000, // 30 seconds
    idleTimeout: config?.idleTimeout || 600000, // 10 minutes
    validationInterval: config?.validationInterval || 60000, // 1 minute
  });

  const [errors, setErrors] = useState<Partial<Record<keyof PoolConfig, string>>>({});

  const handleChange = (field: keyof PoolConfig, value: any) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    // Clear error when user starts typing
    if (errors[field]) {
      setErrors((prev) => ({ ...prev, [field]: undefined }));
    }
  };

  const validateForm = (): boolean => {
    const newErrors: Partial<Record<keyof PoolConfig, string>> = {};

    if (!formData.poolId.trim()) {
      newErrors.poolId = 'Pool ID is required';
    }

    if (formData.minConnections < 1) {
      newErrors.minConnections = 'Minimum connections must be at least 1';
    }

    if (formData.maxConnections < formData.minConnections) {
      newErrors.maxConnections = 'Maximum must be greater than or equal to minimum';
    }

    if (formData.connectionTimeout < 1000) {
      newErrors.connectionTimeout = 'Connection timeout must be at least 1000ms';
    }

    if (formData.idleTimeout < 10000) {
      newErrors.idleTimeout = 'Idle timeout must be at least 10000ms';
    }

    if (formData.validationInterval < 10000) {
      newErrors.validationInterval = 'Validation interval must be at least 10000ms';
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

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(0)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Pool Identification */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-dark-100">Pool Identification</h3>

        <Input
          label="Pool ID"
          value={formData.poolId}
          onChange={(e) => handleChange('poolId', e.target.value)}
          error={errors.poolId}
          placeholder="e.g., main-pool"
          disabled={!!config} // Can't change ID when editing
          helperText={config ? 'Pool ID cannot be changed' : undefined}
          required
          fullWidth
        />
      </div>

      {/* Connection Limits */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-dark-100">Connection Limits</h3>

        <div className="grid grid-cols-2 gap-4">
          <Input
            label="Minimum Connections"
            type="number"
            min="1"
            value={formData.minConnections}
            onChange={(e) => handleChange('minConnections', parseInt(e.target.value))}
            error={errors.minConnections}
            helperText="Always maintained"
            fullWidth
          />

          <Input
            label="Maximum Connections"
            type="number"
            min="1"
            value={formData.maxConnections}
            onChange={(e) => handleChange('maxConnections', parseInt(e.target.value))}
            error={errors.maxConnections}
            helperText="Upper limit"
            fullWidth
          />
        </div>

        <div className="bg-dark-700/50 rounded-lg p-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-dark-300">Pool Size Range</span>
            <span className="text-sm font-semibold text-dark-200">
              {formData.minConnections} - {formData.maxConnections}
            </span>
          </div>
          <div className="w-full bg-dark-700 rounded-full h-2">
            <div
              className="h-full bg-rusty-500 rounded-full"
              style={{
                width: `${
                  (formData.minConnections / formData.maxConnections) * 100
                }%`,
              }}
            />
          </div>
          <div className="flex justify-between text-xs text-dark-400 mt-1">
            <span>Min</span>
            <span>Max</span>
          </div>
        </div>
      </div>

      {/* Timeout Settings */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-dark-100">Timeout Settings</h3>

        <Input
          label="Connection Timeout (ms)"
          type="number"
          min="1000"
          step="1000"
          value={formData.connectionTimeout}
          onChange={(e) =>
            handleChange('connectionTimeout', parseInt(e.target.value))
          }
          error={errors.connectionTimeout}
          helperText={`Timeout for acquiring connections (${formatDuration(
            formData.connectionTimeout
          )})`}
          fullWidth
        />

        <Input
          label="Idle Timeout (ms)"
          type="number"
          min="10000"
          step="1000"
          value={formData.idleTimeout}
          onChange={(e) => handleChange('idleTimeout', parseInt(e.target.value))}
          error={errors.idleTimeout}
          helperText={`Close idle connections after (${formatDuration(
            formData.idleTimeout
          )})`}
          fullWidth
        />

        <Input
          label="Validation Interval (ms)"
          type="number"
          min="10000"
          step="1000"
          value={formData.validationInterval}
          onChange={(e) =>
            handleChange('validationInterval', parseInt(e.target.value))
          }
          error={errors.validationInterval}
          helperText={`Validate connections every (${formatDuration(
            formData.validationInterval
          )})`}
          fullWidth
        />
      </div>

      {/* Recommendations */}
      <div className="bg-info-500/10 border border-info-500/30 rounded-lg p-4">
        <h4 className="text-sm font-semibold text-info-400 mb-2">
          Recommendations
        </h4>
        <ul className="text-xs text-info-400/80 space-y-1 list-disc list-inside">
          <li>Set minimum connections based on typical load</li>
          <li>Keep maximum connections below database limits</li>
          <li>Use shorter timeouts for high-traffic applications</li>
          <li>Validate connections regularly to avoid stale connections</li>
        </ul>
      </div>

      {/* Actions */}
      <div className="flex justify-end gap-3 pt-4 border-t border-dark-700">
        <Button variant="ghost" onClick={onCancel} disabled={loading}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" loading={loading}>
          {config ? 'Update' : 'Create'} Pool
        </Button>
      </div>
    </form>
  );
}
