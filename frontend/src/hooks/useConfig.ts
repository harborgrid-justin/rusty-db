import { useState, useEffect, useCallback, useMemo } from 'react';
import { configService } from '../services/configService';
import type {
  ConfigSetting,
  ConfigCategory,
  ConfigChange,
  ConfigHistoryEntry,
  ConfigValidationResult,
  ConfigRecommendation,
  SystemResources,
} from '../services/configService';
import { getErrorMessage } from '../services/api';

// ============================================================================
// useConfig Hook - Configuration management with validation
// ============================================================================

interface UseConfigOptions {
  category?: ConfigCategory;
  autoRefresh?: boolean;
  refreshInterval?: number;
}

interface UseConfigReturn {
  settings: ConfigSetting[];
  loading: boolean;
  error: string | null;
  pendingChanges: Map<string, unknown>;
  dirtySettings: Set<string>;
  hasPendingChanges: boolean;
  requiresRestart: boolean;

  // Operations
  updateSetting: (key: string, value: unknown) => void;
  resetSetting: (key: string) => Promise<void>;
  resetCategory: (category: ConfigCategory) => Promise<void>;
  applyChanges: (comment?: string) => Promise<void>;
  discardChanges: () => void;
  validateChanges: () => Promise<ConfigValidationResult | null>;
  refresh: () => Promise<void>;

  // Utilities
  getSetting: (key: string) => ConfigSetting | undefined;
  getSettingValue: (key: string) => unknown;
  isDirty: (key: string) => boolean;
  getValidationError: (key: string) => string | undefined;
}

export function useConfig(options: UseConfigOptions = {}): UseConfigReturn {
  const { category, autoRefresh = false, refreshInterval = 30000 } = options;

  // State
  const [settings, setSettings] = useState<ConfigSetting[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pendingChanges, setPendingChanges] = useState<Map<string, unknown>>(new Map());
  const [validationErrors, setValidationErrors] = useState<Map<string, string>>(new Map());

  // Fetch configuration
  const fetchConfig = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      const response = category
        ? await configService.getConfigCategory(category)
        : await configService.getConfig();

      if (response.success && response.data) {
        setSettings(response.data);
      } else {
        setError(response.error?.message || 'Failed to fetch configuration');
      }
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [category]);

  // Initial fetch
  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  // Auto-refresh
  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(fetchConfig, refreshInterval);
    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval, fetchConfig]);

  // Derived state
  const dirtySettings = useMemo(() => {
    return new Set(pendingChanges.keys());
  }, [pendingChanges]);

  const hasPendingChanges = pendingChanges.size > 0;

  const requiresRestart = useMemo(() => {
    return settings.some(
      (setting) => dirtySettings.has(setting.key) && setting.requiresRestart
    );
  }, [settings, dirtySettings]);

  // Update a setting value (local only, not saved)
  const updateSetting = useCallback((key: string, value: unknown) => {
    const setting = settings.find((s) => s.key === key);
    if (!setting) return;

    // Validate the value
    let validationError: string | undefined;

    if (setting.dataType === 'number') {
      const numValue = Number(value);
      if (isNaN(numValue)) {
        validationError = 'Must be a valid number';
      } else if (setting.minValue !== undefined && numValue < setting.minValue) {
        validationError = `Must be at least ${setting.minValue}`;
      } else if (setting.maxValue !== undefined && numValue > setting.maxValue) {
        validationError = `Must be at most ${setting.maxValue}`;
      }
    } else if (setting.dataType === 'enum') {
      if (setting.allowedValues && !setting.allowedValues.includes(value)) {
        validationError = `Must be one of: ${setting.allowedValues.join(', ')}`;
      }
    }

    // Update validation errors
    setValidationErrors((prev) => {
      const next = new Map(prev);
      if (validationError) {
        next.set(key, validationError);
      } else {
        next.delete(key);
      }
      return next;
    });

    // Update pending changes
    setPendingChanges((prev) => {
      const next = new Map(prev);
      if (value === setting.currentValue) {
        next.delete(key);
      } else {
        next.set(key, value);
      }
      return next;
    });
  }, [settings]);

  // Reset a single setting to default
  const resetSetting = useCallback(async (key: string) => {
    try {
      setError(null);
      const response = await configService.resetToDefault(key);

      if (response.success && response.data) {
        // Update local state
        setSettings((prev) =>
          prev.map((s) => (s.key === key ? response.data! : s))
        );
        // Remove from pending changes
        setPendingChanges((prev) => {
          const next = new Map(prev);
          next.delete(key);
          return next;
        });
      } else {
        setError(response.error?.message || 'Failed to reset setting');
      }
    } catch (err) {
      setError(getErrorMessage(err));
    }
  }, []);

  // Reset entire category to defaults
  const resetCategory = useCallback(async (cat: ConfigCategory) => {
    try {
      setError(null);
      const response = await configService.resetCategoryToDefault(cat);

      if (response.success && response.data) {
        // Update local state
        const resetKeys = new Set(response.data.map((s) => s.key));
        setSettings((prev) =>
          prev.map((s) => {
            const resetSetting = response.data!.find((rs) => rs.key === s.key);
            return resetSetting || s;
          })
        );
        // Remove from pending changes
        setPendingChanges((prev) => {
          const next = new Map(prev);
          resetKeys.forEach((key) => next.delete(key));
          return next;
        });
      } else {
        setError(response.error?.message || 'Failed to reset category');
      }
    } catch (err) {
      setError(getErrorMessage(err));
    }
  }, []);

  // Validate pending changes
  const validateChanges = useCallback(async (): Promise<ConfigValidationResult | null> => {
    if (pendingChanges.size === 0) return null;

    try {
      const changes = Array.from(pendingChanges.entries()).map(([key, value]) => ({
        key,
        value,
      }));

      const response = await configService.validateConfig(changes);

      if (response.success && response.data) {
        // Update validation errors from server
        const serverErrors = new Map(
          response.data.errors.map((err) => [err.key, err.message])
        );
        setValidationErrors(serverErrors);
        return response.data;
      }

      return null;
    } catch (err) {
      setError(getErrorMessage(err));
      return null;
    }
  }, [pendingChanges]);

  // Apply pending changes
  const applyChanges = useCallback(async (comment?: string) => {
    if (pendingChanges.size === 0) return;

    try {
      setLoading(true);
      setError(null);

      // Validate first
      const validation = await validateChanges();
      if (validation && validation.errors.length > 0) {
        setError('Please fix validation errors before applying');
        setLoading(false);
        return;
      }

      const changes = Array.from(pendingChanges.entries()).map(([key, value]) => ({
        key,
        value,
      }));

      const response = await configService.updateConfigBatch({
        changes,
        comment,
        applyImmediately: true,
      });

      if (response.success) {
        // Refresh configuration
        await fetchConfig();
        // Clear pending changes
        setPendingChanges(new Map());
        setValidationErrors(new Map());
      } else {
        setError(response.error?.message || 'Failed to apply changes');
      }
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [pendingChanges, validateChanges, fetchConfig]);

  // Discard pending changes
  const discardChanges = useCallback(() => {
    setPendingChanges(new Map());
    setValidationErrors(new Map());
  }, []);

  // Refresh configuration
  const refresh = useCallback(async () => {
    await fetchConfig();
  }, [fetchConfig]);

  // Utility functions
  const getSetting = useCallback(
    (key: string): ConfigSetting | undefined => {
      return settings.find((s) => s.key === key);
    },
    [settings]
  );

  const getSettingValue = useCallback(
    (key: string): unknown => {
      const pendingValue = pendingChanges.get(key);
      if (pendingValue !== undefined) return pendingValue;

      const setting = getSetting(key);
      return setting?.currentValue;
    },
    [pendingChanges, getSetting]
  );

  const isDirty = useCallback(
    (key: string): boolean => {
      return dirtySettings.has(key);
    },
    [dirtySettings]
  );

  const getValidationError = useCallback(
    (key: string): string | undefined => {
      return validationErrors.get(key);
    },
    [validationErrors]
  );

  return {
    settings,
    loading,
    error,
    pendingChanges,
    dirtySettings,
    hasPendingChanges,
    requiresRestart,
    updateSetting,
    resetSetting,
    resetCategory,
    applyChanges,
    discardChanges,
    validateChanges,
    refresh,
    getSetting,
    getSettingValue,
    isDirty,
    getValidationError,
  };
}

// ============================================================================
// useConfigHistory Hook
// ============================================================================

interface UseConfigHistoryReturn {
  history: ConfigHistoryEntry[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  rollback: (historyId: string) => Promise<void>;
}

export function useConfigHistory(limit = 50): UseConfigHistoryReturn {
  const [history, setHistory] = useState<ConfigHistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchHistory = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      const response = await configService.getConfigHistory(limit);

      if (response.success && response.data) {
        setHistory(response.data);
      } else {
        setError(response.error?.message || 'Failed to fetch history');
      }
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [limit]);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  const rollback = useCallback(async (historyId: string) => {
    try {
      setLoading(true);
      setError(null);

      const response = await configService.rollbackConfig(historyId);

      if (response.success) {
        await fetchHistory();
      } else {
        setError(response.error?.message || 'Failed to rollback configuration');
      }
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [fetchHistory]);

  return {
    history,
    loading,
    error,
    refresh: fetchHistory,
    rollback,
  };
}

// ============================================================================
// useConfigRecommendations Hook
// ============================================================================

interface UseConfigRecommendationsReturn {
  recommendations: ConfigRecommendation[];
  systemResources: SystemResources | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  applyRecommendation: (key: string) => void;
}

export function useConfigRecommendations(): UseConfigRecommendationsReturn {
  const [recommendations, setRecommendations] = useState<ConfigRecommendation[]>([]);
  const [systemResources, setSystemResources] = useState<SystemResources | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      const [recommendationsResponse, resourcesResponse] = await Promise.all([
        configService.getRecommendations(),
        configService.getSystemResources(),
      ]);

      if (recommendationsResponse.success && recommendationsResponse.data) {
        setRecommendations(recommendationsResponse.data);
      }

      if (resourcesResponse.success && resourcesResponse.data) {
        setSystemResources(resourcesResponse.data);
      }
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const applyRecommendation = useCallback(() => {
    // This would be used in conjunction with useConfig to apply a recommendation
    // Implementation depends on parent component
  }, []);

  return {
    recommendations,
    systemResources,
    loading,
    error,
    refresh: fetchData,
    applyRecommendation,
  };
}

// ============================================================================
// usePendingRestart Hook
// ============================================================================

interface UsePendingRestartReturn {
  pendingChanges: ConfigChange[];
  requiresRestart: boolean;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function usePendingRestart(): UsePendingRestartReturn {
  const [pendingChanges, setPendingChanges] = useState<ConfigChange[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchPendingChanges = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      const response = await configService.getPendingRestartChanges();

      if (response.success && response.data) {
        setPendingChanges(response.data);
      } else {
        setError(response.error?.message || 'Failed to fetch pending changes');
      }
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchPendingChanges();
  }, [fetchPendingChanges]);

  return {
    pendingChanges,
    requiresRestart: pendingChanges.length > 0,
    loading,
    error,
    refresh: fetchPendingChanges,
  };
}
