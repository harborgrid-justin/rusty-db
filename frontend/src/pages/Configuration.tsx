import React, { useState } from 'react';
import { useConfig, useConfigHistory, usePendingRestart } from '../hooks/useConfig';
import { ConfigSection } from '../components/config/ConfigSection';
import { ConfigInput } from '../components/config/ConfigInput';
import { ConfigDiff } from '../components/config/ConfigDiff';
import { ConfigHistory } from '../components/config/ConfigHistory';
import { ImportExportConfig } from '../components/config/ImportExportConfig';
import { RestartBanner } from '../components/config/RestartBanner';
import { configService } from '../services/configService';
import type { ConfigCategory } from '../services/configService';

// ============================================================================
// Configuration Page - General configuration overview
// ============================================================================

export function Configuration() {
  const {
    settings,
    loading,
    error,
    pendingChanges,
    hasPendingChanges,
    updateSetting,
    resetSetting,
    resetCategory,
    applyChanges,
    discardChanges,
    getSettingValue,
    isDirty,
    getValidationError,
    refresh,
  } = useConfig();

  const { history, loading: historyLoading, rollback } = useConfigHistory(20);
  const { pendingChanges: restartChanges, refresh: refreshRestart } = usePendingRestart();

  const [activeTab, setActiveTab] = useState<'settings' | 'history' | 'import-export'>('settings');
  const [searchQuery, setSearchQuery] = useState('');
  const [showDiffModal, setShowDiffModal] = useState(false);

  // Group settings by category
  const settingsByCategory = settings.reduce((acc, setting) => {
    if (!acc[setting.category]) {
      acc[setting.category] = [];
    }
    acc[setting.category].push(setting);
    return acc;
  }, {} as Record<ConfigCategory, typeof settings>);

  // Filter settings by search query
  const filteredSettings = searchQuery
    ? settings.filter(
        (s) =>
          s.key.toLowerCase().includes(searchQuery.toLowerCase()) ||
          s.description.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : settings;

  const handleApplyChanges = async () => {
    await applyChanges();
    await refresh();
    await refreshRestart();
    setShowDiffModal(false);
  };

  const handleExport = async (includeDefaults: boolean) => {
    const response = await configService.exportConfig(includeDefaults);
    if (response.success && response.data) {
      // Download as JSON file
      const blob = new Blob([JSON.stringify(response.data, null, 2)], {
        type: 'application/json',
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `rustydb-config-${new Date().toISOString().split('T')[0]}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    }
  };

  const handleImport = async (config: Record<string, unknown>) => {
    const result = await configService.importConfig(config, true);
    if (result.success && result.data) {
      await refresh();
      return result.data;
    }
    return null;
  };

  const handleRollback = async (historyId: string) => {
    await rollback(historyId);
    await refresh();
    await refreshRestart();
  };

  const categories: Array<{ key: ConfigCategory; label: string; description: string }> = [
    { key: 'general', label: 'General', description: 'Basic database settings' },
    { key: 'connection', label: 'Connections', description: 'Connection pool and timeout settings' },
    { key: 'memory', label: 'Memory', description: 'Memory allocation and buffer settings' },
    { key: 'performance', label: 'Performance', description: 'Query execution and optimization' },
    { key: 'wal', label: 'Write-Ahead Logging', description: 'WAL and checkpoint configuration' },
    { key: 'security', label: 'Security', description: 'Authentication and encryption settings' },
    { key: 'logging', label: 'Logging', description: 'Log output and verbosity' },
    { key: 'replication', label: 'Replication', description: 'Replication and standby settings' },
    { key: 'maintenance', label: 'Maintenance', description: 'Vacuum and analyze settings' },
  ];

  if (loading && settings.length === 0) {
    return (
      <div className="configuration-page loading">
        <div className="spinner" />
        <p>Loading configuration...</p>
      </div>
    );
  }

  return (
    <div className="configuration-page">
      {/* Restart Banner */}
      {restartChanges.length > 0 && (
        <RestartBanner
          pendingChanges={restartChanges}
          onRestartNow={() => {
            console.log('Restart database');
          }}
          onScheduleRestart={() => {
            console.log('Schedule restart');
          }}
          onDismiss={() => {
            console.log('Dismiss banner');
          }}
        />
      )}

      {/* Page Header */}
      <div className="configuration-header">
        <div>
          <h1 className="configuration-title">Database Configuration</h1>
          <p className="configuration-subtitle">
            Manage database settings and performance tuning
          </p>
        </div>

        {/* Quick Actions */}
        <div className="configuration-quick-actions">
          <button className="config-action-btn" onClick={() => setActiveTab('import-export')}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path
                d="M8 2V10M8 10L5 7M8 10L11 7"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <path d="M2 14H14" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
            Import/Export
          </button>

          <button className="config-action-btn" onClick={() => setActiveTab('history')}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="1.5" />
              <path
                d="M8 4V8L11 9"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            </svg>
            History
          </button>

          {hasPendingChanges && (
            <>
              <button
                className="config-action-btn primary"
                onClick={() => setShowDiffModal(true)}
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M13 4L6 11L3 8"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
                Apply Changes ({pendingChanges.size})
              </button>

              <button className="config-action-btn" onClick={discardChanges}>
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M4 4L12 12M12 4L4 12"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                  />
                </svg>
                Discard
              </button>
            </>
          )}
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className="configuration-error">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
            <path d="M8 4V8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <circle cx="8" cy="11" r="0.5" fill="currentColor" />
          </svg>
          {error}
        </div>
      )}

      {/* Tabs */}
      <div className="configuration-tabs">
        <button
          className={`configuration-tab ${activeTab === 'settings' ? 'active' : ''}`}
          onClick={() => setActiveTab('settings')}
        >
          Settings
        </button>
        <button
          className={`configuration-tab ${activeTab === 'history' ? 'active' : ''}`}
          onClick={() => setActiveTab('history')}
        >
          History
        </button>
        <button
          className={`configuration-tab ${activeTab === 'import-export' ? 'active' : ''}`}
          onClick={() => setActiveTab('import-export')}
        >
          Import/Export
        </button>
      </div>

      {/* Tab Content */}
      <div className="configuration-content">
        {/* Settings Tab */}
        {activeTab === 'settings' && (
          <>
            {/* Search */}
            <div className="configuration-search">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.5" />
                <path
                  d="M11 11L14 14"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                />
              </svg>
              <input
                type="text"
                placeholder="Search settings..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="configuration-search-input"
              />
              {searchQuery && (
                <button
                  className="configuration-search-clear"
                  onClick={() => setSearchQuery('')}
                >
                  <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                    <path
                      d="M3 3L11 11M11 3L3 11"
                      stroke="currentColor"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                    />
                  </svg>
                </button>
              )}
            </div>

            {/* Category Sections */}
            {searchQuery ? (
              // Search Results
              <ConfigSection
                title="Search Results"
                description={`${filteredSettings.length} setting(s) found`}
                category="general"
                settings={filteredSettings}
                defaultExpanded={true}
                showResetButton={false}
              >
                {filteredSettings.map((setting) => (
                  <ConfigInput
                    key={setting.key}
                    setting={setting}
                    value={getSettingValue(setting.key)}
                    onChange={(value) => updateSetting(setting.key, value)}
                    onReset={() => resetSetting(setting.key)}
                    error={getValidationError(setting.key)}
                    isDirty={isDirty(setting.key)}
                  />
                ))}
              </ConfigSection>
            ) : (
              // Category Sections
              categories.map(
                (category) =>
                  settingsByCategory[category.key] && (
                    <ConfigSection
                      key={category.key}
                      title={category.label}
                      description={category.description}
                      category={category.key}
                      settings={settingsByCategory[category.key]}
                      onResetSection={() => resetCategory(category.key)}
                    >
                      {settingsByCategory[category.key].map((setting) => (
                        <ConfigInput
                          key={setting.key}
                          setting={setting}
                          value={getSettingValue(setting.key)}
                          onChange={(value) => updateSetting(setting.key, value)}
                          onReset={() => resetSetting(setting.key)}
                          error={getValidationError(setting.key)}
                          isDirty={isDirty(setting.key)}
                        />
                      ))}
                    </ConfigSection>
                  )
              )
            )}
          </>
        )}

        {/* History Tab */}
        {activeTab === 'history' && (
          <ConfigHistory
            history={history}
            onRollback={handleRollback}
            loading={historyLoading}
          />
        )}

        {/* Import/Export Tab */}
        {activeTab === 'import-export' && (
          <ImportExportConfig onExport={handleExport} onImport={handleImport} />
        )}
      </div>

      {/* Diff Modal */}
      {showDiffModal && hasPendingChanges && (
        <div className="configuration-modal">
          <div className="configuration-modal-overlay" onClick={() => setShowDiffModal(false)} />
          <div className="configuration-modal-content">
            <ConfigDiff
              changes={Array.from(pendingChanges.entries()).map(([key, newValue]) => {
                const setting = settings.find((s) => s.key === key)!;
                return {
                  setting,
                  oldValue: setting.currentValue,
                  newValue,
                };
              })}
              onApply={handleApplyChanges}
              onCancel={() => setShowDiffModal(false)}
            />
          </div>
        </div>
      )}

      <style>{`
        .configuration-page {
          min-height: 100vh;
          background: var(--background-color, #f9fafb);
        }

        .configuration-page.loading {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 48px;
        }

        .spinner {
          width: 40px;
          height: 40px;
          border: 4px solid var(--border-color, #e5e7eb);
          border-top-color: var(--primary-color, #3b82f6);
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
          margin-bottom: 16px;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }

        .configuration-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          padding: 24px;
          background: var(--surface-color, #ffffff);
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .configuration-title {
          margin: 0 0 4px 0;
          font-size: 24px;
          font-weight: 700;
          color: var(--text-primary, #111827);
        }

        .configuration-subtitle {
          margin: 0;
          font-size: 14px;
          color: var(--text-secondary, #6b7280);
        }

        .configuration-quick-actions {
          display: flex;
          gap: 8px;
        }

        .config-action-btn {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 10px 16px;
          font-size: 14px;
          font-weight: 500;
          color: var(--text-primary, #111827);
          background: var(--surface-secondary, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .config-action-btn:hover {
          background: var(--surface-hover, #f3f4f6);
        }

        .config-action-btn.primary {
          background: var(--primary-color, #3b82f6);
          border-color: var(--primary-color, #3b82f6);
          color: #ffffff;
        }

        .config-action-btn.primary:hover {
          background: var(--primary-color-dark, #2563eb);
        }

        .configuration-error {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 24px;
          background: var(--error-bg, #fee2e2);
          color: var(--error-text, #991b1b);
          border-bottom: 1px solid var(--error-border, #fecaca);
        }

        .configuration-tabs {
          display: flex;
          gap: 4px;
          padding: 0 24px;
          background: var(--surface-color, #ffffff);
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .configuration-tab {
          padding: 12px 20px;
          font-size: 14px;
          font-weight: 500;
          color: var(--text-secondary, #6b7280);
          background: none;
          border: none;
          border-bottom: 2px solid transparent;
          cursor: pointer;
          transition: all 0.2s;
        }

        .configuration-tab:hover {
          color: var(--text-primary, #111827);
        }

        .configuration-tab.active {
          color: var(--primary-color, #3b82f6);
          border-bottom-color: var(--primary-color, #3b82f6);
        }

        .configuration-content {
          padding: 24px;
          max-width: 1400px;
          margin: 0 auto;
        }

        .configuration-search {
          position: relative;
          display: flex;
          align-items: center;
          margin-bottom: 24px;
          padding: 0 16px;
          background: var(--surface-color, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
        }

        .configuration-search svg {
          color: var(--text-tertiary, #9ca3af);
        }

        .configuration-search-input {
          flex: 1;
          padding: 12px 12px 12px 8px;
          font-size: 14px;
          color: var(--text-primary, #111827);
          background: none;
          border: none;
          outline: none;
        }

        .configuration-search-clear {
          padding: 4px;
          background: none;
          border: none;
          color: var(--text-secondary, #6b7280);
          cursor: pointer;
          transition: color 0.2s;
        }

        .configuration-search-clear:hover {
          color: var(--text-primary, #111827);
        }

        .configuration-modal {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          z-index: 50;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 24px;
        }

        .configuration-modal-overlay {
          position: absolute;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background: rgba(0, 0, 0, 0.5);
          backdrop-filter: blur(4px);
        }

        .configuration-modal-content {
          position: relative;
          max-width: 800px;
          max-height: 90vh;
          overflow-y: auto;
          z-index: 51;
        }

        /* Dark mode */
        @media (prefers-color-scheme: dark) {
          .configuration-page {
            background: var(--background-color, #111827);
          }

          .configuration-header,
          .configuration-tabs {
            background: var(--surface-color, #1f2937);
            border-bottom-color: var(--border-color, #374151);
          }

          .configuration-search {
            background: var(--surface-color, #1f2937);
            border-color: var(--border-color, #374151);
          }

          .config-action-btn {
            background: var(--surface-secondary, #111827);
            border-color: var(--border-color, #374151);
            color: var(--text-primary, #f9fafb);
          }

          .config-action-btn:hover {
            background: var(--surface-hover, #1f2937);
          }

          .configuration-error {
            background: var(--error-bg, #7f1d1d);
            color: var(--error-text, #fca5a5);
            border-bottom-color: var(--error-border, #991b1b);
          }
        }
      `}</style>
    </div>
  );
}

export default Configuration;
