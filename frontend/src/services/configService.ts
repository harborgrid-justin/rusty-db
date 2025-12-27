import { get, post, put } from './api';
import type { ApiResponse, DatabaseConfig } from '../types';

// ============================================================================
// Configuration Service - API interactions for database configuration
// ============================================================================

// ============================================================================
// Configuration Types
// ============================================================================

export interface ConfigSetting {
  key: string;
  value: unknown;
  dataType: 'string' | 'number' | 'boolean' | 'enum';
  category: ConfigCategory;
  description: string;
  defaultValue: unknown;
  currentValue: unknown;
  minValue?: number;
  maxValue?: number;
  allowedValues?: unknown[];
  requiresRestart: boolean;
  unit?: string;
  isDirty?: boolean;
  validationError?: string;
}

export type ConfigCategory =
  | 'general'
  | 'performance'
  | 'security'
  | 'logging'
  | 'replication'
  | 'maintenance'
  | 'connection'
  | 'memory'
  | 'wal'
  | 'query';

export interface ConfigChange {
  key: string;
  oldValue: unknown;
  newValue: unknown;
  timestamp: string;
  userId?: string;
  username?: string;
  requiresRestart: boolean;
}

export interface ConfigHistoryEntry {
  id: string;
  changes: ConfigChange[];
  timestamp: string;
  userId: string;
  username: string;
  comment?: string;
  appliedAt?: string;
  status: 'pending' | 'applied' | 'failed' | 'rolled_back';
}

export interface ConfigValidationResult {
  valid: boolean;
  errors: ConfigValidationError[];
  warnings: ConfigValidationWarning[];
}

export interface ConfigValidationError {
  key: string;
  message: string;
  code: string;
}

export interface ConfigValidationWarning {
  key: string;
  message: string;
  impact: 'low' | 'medium' | 'high';
}

export interface ConfigRecommendation {
  key: string;
  recommendedValue: unknown;
  currentValue: unknown;
  reason: string;
  impact: 'performance' | 'security' | 'reliability';
  priority: 'low' | 'medium' | 'high';
}

export interface SystemResources {
  totalMemory: number;
  availableMemory: number;
  cpuCores: number;
  diskSize: number;
  diskType: 'ssd' | 'hdd' | 'nvme';
}

export interface ConfigExport {
  version: string;
  exportedAt: string;
  settings: Record<string, unknown>;
  metadata?: Record<string, unknown>;
}

export interface BatchUpdateRequest {
  changes: Array<{
    key: string;
    value: unknown;
  }>;
  comment?: string;
  applyImmediately?: boolean;
}

// ============================================================================
// Configuration Service
// ============================================================================

class ConfigurationService {
  /**
   * Get all configuration settings
   */
  async getConfig(): Promise<ApiResponse<ConfigSetting[]>> {
    return get<ConfigSetting[]>('/config');
  }

  /**
   * Get configuration by category
   */
  async getConfigCategory(category: ConfigCategory): Promise<ApiResponse<ConfigSetting[]>> {
    return get<ConfigSetting[]>(`/config/category/${category}`);
  }

  /**
   * Get specific configuration setting
   */
  async getConfigSetting(key: string): Promise<ApiResponse<ConfigSetting>> {
    return get<ConfigSetting>(`/config/${key}`);
  }

  /**
   * Get full database configuration object
   */
  async getDatabaseConfig(): Promise<ApiResponse<DatabaseConfig>> {
    return get<DatabaseConfig>('/config/database');
  }

  /**
   * Update a single configuration setting
   */
  async updateConfig(key: string, value: unknown, comment?: string): Promise<ApiResponse<ConfigSetting>> {
    return put<ConfigSetting>(`/config/${key}`, { value, comment });
  }

  /**
   * Update multiple configuration settings in batch
   */
  async updateConfigBatch(request: BatchUpdateRequest): Promise<ApiResponse<ConfigChange[]>> {
    return post<ConfigChange[]>('/config/batch', request);
  }

  /**
   * Validate configuration changes before applying
   */
  async validateConfig(changes: Array<{ key: string; value: unknown }>): Promise<ApiResponse<ConfigValidationResult>> {
    return post<ConfigValidationResult>('/config/validate', { changes });
  }

  /**
   * Get configuration change history
   */
  async getConfigHistory(limit = 50, offset = 0): Promise<ApiResponse<ConfigHistoryEntry[]>> {
    return get<ConfigHistoryEntry[]>(`/config/history?limit=${limit}&offset=${offset}`);
  }

  /**
   * Get history for a specific configuration setting
   */
  async getConfigSettingHistory(key: string): Promise<ApiResponse<ConfigChange[]>> {
    return get<ConfigChange[]>(`/config/${key}/history`);
  }

  /**
   * Reset a configuration setting to its default value
   */
  async resetToDefault(key: string): Promise<ApiResponse<ConfigSetting>> {
    return post<ConfigSetting>(`/config/${key}/reset`);
  }

  /**
   * Reset multiple settings to defaults
   */
  async resetCategoryToDefault(category: ConfigCategory): Promise<ApiResponse<ConfigSetting[]>> {
    return post<ConfigSetting[]>(`/config/category/${category}/reset`);
  }

  /**
   * Export configuration to file
   */
  async exportConfig(includeDefaults = false): Promise<ApiResponse<ConfigExport>> {
    return get<ConfigExport>(`/config/export?includeDefaults=${includeDefaults}`);
  }

  /**
   * Import configuration from file
   */
  async importConfig(config: ConfigExport, validate = true): Promise<ApiResponse<ConfigValidationResult>> {
    return post<ConfigValidationResult>('/config/import', { config, validate });
  }

  /**
   * Apply pending configuration changes (requires restart if needed)
   */
  async applyPendingChanges(): Promise<ApiResponse<{ requiresRestart: boolean; appliedChanges: string[] }>> {
    return post('/config/apply');
  }

  /**
   * Rollback to a previous configuration state
   */
  async rollbackConfig(historyId: string): Promise<ApiResponse<ConfigChange[]>> {
    return post<ConfigChange[]>(`/config/rollback/${historyId}`);
  }

  /**
   * Get system resources for configuration recommendations
   */
  async getSystemResources(): Promise<ApiResponse<SystemResources>> {
    return get<SystemResources>('/config/system-resources');
  }

  /**
   * Get configuration recommendations based on system resources
   */
  async getRecommendations(): Promise<ApiResponse<ConfigRecommendation[]>> {
    return get<ConfigRecommendation[]>('/config/recommendations');
  }

  /**
   * Get pending configuration changes that require restart
   */
  async getPendingRestartChanges(): Promise<ApiResponse<ConfigChange[]>> {
    return get<ConfigChange[]>('/config/pending-restart');
  }

  /**
   * Test configuration changes without applying
   */
  async testConfig(changes: Array<{ key: string; value: unknown }>): Promise<ApiResponse<{
    success: boolean;
    errors: string[];
    warnings: string[];
    performanceImpact?: {
      memory: number;
      cpu: number;
      disk: number;
    };
  }>> {
    return post('/config/test', { changes });
  }

  /**
   * Get configuration templates for common scenarios
   */
  async getConfigTemplates(): Promise<ApiResponse<Array<{
    name: string;
    description: string;
    scenario: string;
    settings: Record<string, unknown>;
  }>>> {
    return get('/config/templates');
  }

  /**
   * Apply a configuration template
   */
  async applyTemplate(templateName: string): Promise<ApiResponse<ConfigChange[]>> {
    return post<ConfigChange[]>(`/config/templates/${templateName}/apply`);
  }

  /**
   * Search configuration settings
   */
  async searchConfig(query: string): Promise<ApiResponse<ConfigSetting[]>> {
    return get<ConfigSetting[]>(`/config/search?q=${encodeURIComponent(query)}`);
  }

  /**
   * Get configuration diff between current and default
   */
  async getConfigDiff(): Promise<ApiResponse<Array<{
    key: string;
    currentValue: unknown;
    defaultValue: unknown;
    category: ConfigCategory;
  }>>> {
    return get('/config/diff');
  }

  /**
   * Get list of available databases for backup/restore operations
   */
  async getAvailableDatabases(): Promise<ApiResponse<string[]>> {
    return get<string[]>('/databases');
  }
}

// ============================================================================
// Export singleton instance
// ============================================================================

export const configService = new ConfigurationService();
export default configService;
