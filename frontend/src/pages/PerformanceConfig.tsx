import React, { useState, useEffect } from 'react';
import { useConfig, useConfigRecommendations } from '../hooks/useConfig';
import { ConfigSection } from '../components/config/ConfigSection';
import { ConfigInput } from '../components/config/ConfigInput';
import { MemoryConfigSlider } from '../components/config/MemoryConfigSlider';
import { RestartBanner } from '../components/config/RestartBanner';
import { usePendingRestart } from '../hooks/useConfig';

// ============================================================================
// PerformanceConfig Page - Performance settings with recommendations
// ============================================================================

export function PerformanceConfig() {
  const {
    settings,
    loading,
    error,
    hasPendingChanges,
    updateSetting,
    resetSetting,
    resetCategory,
    applyChanges,
    discardChanges,
    getSettingValue,
    isDirty,
    getValidationError,
  } = useConfig({ category: 'performance' });

  const {
    recommendations,
    systemResources,
    loading: recommendationsLoading,
  } = useConfigRecommendations();

  const { pendingChanges: restartChanges } = usePendingRestart();

  const [activeSection, setActiveSection] = useState<'memory' | 'connections' | 'query' | 'checkpoint' | 'wal'>('memory');

  // Memory settings for visual slider
  const memoryAllocations = systemResources ? [
    {
      name: 'Shared Buffers',
      key: 'shared_buffers',
      value: Number(getSettingValue('shared_buffers')) || 0,
      color: '#3b82f6',
      description: 'Main database cache',
    },
    {
      name: 'Effective Cache Size',
      key: 'effective_cache_size',
      value: Number(getSettingValue('effective_cache_size')) || 0,
      color: '#10b981',
      description: 'Expected OS cache size',
    },
    {
      name: 'Work Memory',
      key: 'work_mem',
      value: Number(getSettingValue('work_mem')) || 0,
      color: '#f59e0b',
      description: 'Per-operation memory',
    },
    {
      name: 'Maintenance Work Memory',
      key: 'maintenance_work_mem',
      value: Number(getSettingValue('maintenance_work_mem')) || 0,
      color: '#8b5cf6',
      description: 'Vacuum and index operations',
    },
  ] : [];

  const handleMemoryChange = (key: string, value: number) => {
    updateSetting(key, value);
  };

  const handleApplyRecommendation = (key: string, value: unknown) => {
    updateSetting(key, value);
  };

  const handleApplyAllRecommendations = () => {
    recommendations.forEach((rec) => {
      updateSetting(rec.key, rec.recommendedValue);
    });
  };

  const handleSaveChanges = async () => {
    await applyChanges();
  };

  const formatBytes = (bytes: number): string => {
    const gb = bytes / (1024 * 1024 * 1024);
    if (gb >= 1) return `${gb.toFixed(2)} GB`;
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(0)} MB`;
  };

  const getRecommendationsForSection = (section: string) => {
    const sectionKeys: Record<string, string[]> = {
      memory: ['shared_buffers', 'effective_cache_size', 'work_mem', 'maintenance_work_mem'],
      connections: ['max_connections', 'connection_timeout', 'idle_timeout'],
      query: ['parallel_workers', 'random_page_cost', 'seq_page_cost', 'cpu_tuple_cost'],
      checkpoint: ['checkpoint_segments', 'checkpoint_timeout', 'checkpoint_completion_target'],
      wal: ['wal_buffers', 'wal_level', 'max_wal_senders', 'wal_keep_segments'],
    };

    return recommendations.filter((rec) =>
      sectionKeys[section]?.some((key) => rec.key.includes(key))
    );
  };

  if (loading && settings.length === 0) {
    return (
      <div className="performance-config loading">
        <div className="spinner" />
        <p>Loading performance configuration...</p>
      </div>
    );
  }

  return (
    <div className="performance-config">
      {/* Restart Banner */}
      {restartChanges.length > 0 && (
        <RestartBanner
          pendingChanges={restartChanges}
          onRestartNow={() => console.log('Restart database')}
          onScheduleRestart={() => console.log('Schedule restart')}
        />
      )}

      {/* Page Header */}
      <div className="performance-header">
        <div>
          <h1 className="performance-title">Performance Configuration</h1>
          <p className="performance-subtitle">
            Optimize database performance with recommended settings
          </p>
        </div>

        <div className="performance-actions">
          {recommendations.length > 0 && (
            <button
              className="perf-action-btn secondary"
              onClick={handleApplyAllRecommendations}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path
                  d="M8 2L9.5 5.5L13 7L9.5 8.5L8 12L6.5 8.5L3 7L6.5 5.5L8 2Z"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinejoin="round"
                />
              </svg>
              Apply All Recommendations
            </button>
          )}

          {hasPendingChanges && (
            <>
              <button className="perf-action-btn primary" onClick={handleSaveChanges}>
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M13 4L6 11L3 8"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
                Save Changes
              </button>

              <button className="perf-action-btn" onClick={discardChanges}>
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
        <div className="performance-error">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
            <path d="M8 4V8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <circle cx="8" cy="11" r="0.5" fill="currentColor" />
          </svg>
          {error}
        </div>
      )}

      {/* System Resources Overview */}
      {systemResources && (
        <div className="performance-resources">
          <h2 className="performance-resources-title">System Resources</h2>
          <div className="performance-resources-grid">
            <div className="performance-resource-card">
              <div className="resource-icon">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <rect x="4" y="4" width="16" height="16" rx="2" stroke="currentColor" strokeWidth="2" />
                  <path d="M4 9H20M9 4V20" stroke="currentColor" strokeWidth="2" />
                </svg>
              </div>
              <div className="resource-info">
                <span className="resource-label">Total Memory</span>
                <span className="resource-value">{formatBytes(systemResources.totalMemory)}</span>
              </div>
            </div>

            <div className="performance-resource-card">
              <div className="resource-icon">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <circle cx="12" cy="12" r="8" stroke="currentColor" strokeWidth="2" />
                  <path d="M12 8V12L15 15" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                </svg>
              </div>
              <div className="resource-info">
                <span className="resource-label">CPU Cores</span>
                <span className="resource-value">{systemResources.cpuCores}</span>
              </div>
            </div>

            <div className="performance-resource-card">
              <div className="resource-icon">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <circle cx="12" cy="12" r="8" stroke="currentColor" strokeWidth="2" />
                  <path d="M12 6V12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                </svg>
              </div>
              <div className="resource-info">
                <span className="resource-label">Disk Type</span>
                <span className="resource-value">{systemResources.diskType.toUpperCase()}</span>
              </div>
            </div>

            <div className="performance-resource-card">
              <div className="resource-icon">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <rect x="3" y="6" width="18" height="12" rx="2" stroke="currentColor" strokeWidth="2" />
                  <path d="M7 10H17M7 14H13" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                </svg>
              </div>
              <div className="resource-info">
                <span className="resource-label">Disk Size</span>
                <span className="resource-value">{formatBytes(systemResources.diskSize)}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Section Navigation */}
      <div className="performance-sections">
        <button
          className={`section-btn ${activeSection === 'memory' ? 'active' : ''}`}
          onClick={() => setActiveSection('memory')}
        >
          Memory Settings
        </button>
        <button
          className={`section-btn ${activeSection === 'connections' ? 'active' : ''}`}
          onClick={() => setActiveSection('connections')}
        >
          Connections
        </button>
        <button
          className={`section-btn ${activeSection === 'query' ? 'active' : ''}`}
          onClick={() => setActiveSection('query')}
        >
          Query Execution
        </button>
        <button
          className={`section-btn ${activeSection === 'checkpoint' ? 'active' : ''}`}
          onClick={() => setActiveSection('checkpoint')}
        >
          Checkpoints
        </button>
        <button
          className={`section-btn ${activeSection === 'wal' ? 'active' : ''}`}
          onClick={() => setActiveSection('wal')}
        >
          Write-Ahead Log
        </button>
      </div>

      {/* Content */}
      <div className="performance-content">
        {/* Memory Section */}
        {activeSection === 'memory' && systemResources && (
          <div className="performance-section">
            <MemoryConfigSlider
              totalSystemMemory={systemResources.totalMemory}
              allocations={memoryAllocations}
              onChange={handleMemoryChange}
            />

            {/* Recommendations */}
            {getRecommendationsForSection('memory').length > 0 && (
              <div className="performance-recommendations">
                <h3 className="recommendations-title">Recommendations</h3>
                {getRecommendationsForSection('memory').map((rec) => (
                  <div key={rec.key} className="recommendation-card">
                    <div className="recommendation-header">
                      <code className="recommendation-key">{rec.key}</code>
                      <span className={`recommendation-priority priority-${rec.priority}`}>
                        {rec.priority} priority
                      </span>
                    </div>
                    <p className="recommendation-reason">{rec.reason}</p>
                    <div className="recommendation-values">
                      <div className="recommendation-value current">
                        <span className="label">Current:</span>
                        <code>{String(rec.currentValue)}</code>
                      </div>
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <path
                          d="M3 8H13M13 8L9 4M13 8L9 12"
                          stroke="currentColor"
                          strokeWidth="1.5"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        />
                      </svg>
                      <div className="recommendation-value recommended">
                        <span className="label">Recommended:</span>
                        <code>{String(rec.recommendedValue)}</code>
                      </div>
                    </div>
                    <button
                      className="recommendation-apply-btn"
                      onClick={() => handleApplyRecommendation(rec.key, rec.recommendedValue)}
                    >
                      Apply Recommendation
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Other Sections */}
        {activeSection !== 'memory' && (
          <ConfigSection
            title={activeSection.charAt(0).toUpperCase() + activeSection.slice(1)}
            description={`Configure ${activeSection} settings`}
            category="performance"
            settings={settings.filter((s) => s.key.includes(activeSection))}
            onResetSection={() => resetCategory('performance')}
          >
            {settings
              .filter((s) => s.key.includes(activeSection))
              .map((setting) => (
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
        )}
      </div>

      <style>{`
        .performance-config {
          min-height: 100vh;
          background: var(--background-color, #f9fafb);
        }

        .performance-config.loading {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
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

        .performance-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          padding: 24px;
          background: var(--surface-color, #ffffff);
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .performance-title {
          margin: 0 0 4px 0;
          font-size: 24px;
          font-weight: 700;
          color: var(--text-primary, #111827);
        }

        .performance-subtitle {
          margin: 0;
          font-size: 14px;
          color: var(--text-secondary, #6b7280);
        }

        .performance-actions {
          display: flex;
          gap: 8px;
        }

        .perf-action-btn {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 10px 16px;
          font-size: 14px;
          font-weight: 500;
          border-radius: 6px;
          border: 1px solid;
          cursor: pointer;
          transition: all 0.2s;
        }

        .perf-action-btn.primary {
          background: var(--primary-color, #3b82f6);
          border-color: var(--primary-color, #3b82f6);
          color: #ffffff;
        }

        .perf-action-btn.primary:hover {
          background: var(--primary-color-dark, #2563eb);
        }

        .perf-action-btn.secondary {
          background: var(--surface-secondary, #f9fafb);
          border-color: var(--border-color, #e5e7eb);
          color: var(--text-primary, #111827);
        }

        .perf-action-btn.secondary:hover {
          background: var(--surface-hover, #f3f4f6);
        }

        .perf-action-btn:not(.primary):not(.secondary) {
          background: var(--surface-color, #ffffff);
          border-color: var(--border-color, #d1d5db);
          color: var(--text-primary, #111827);
        }

        .perf-action-btn:not(.primary):not(.secondary):hover {
          background: var(--surface-hover, #f9fafb);
        }

        .performance-error {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 24px;
          background: var(--error-bg, #fee2e2);
          color: var(--error-text, #991b1b);
          border-bottom: 1px solid var(--error-border, #fecaca);
        }

        .performance-resources {
          padding: 24px;
          background: var(--surface-color, #ffffff);
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .performance-resources-title {
          margin: 0 0 16px 0;
          font-size: 16px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .performance-resources-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
          gap: 16px;
        }

        .performance-resource-card {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 16px;
          background: var(--surface-secondary, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
        }

        .resource-icon {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 48px;
          height: 48px;
          background: var(--primary-bg-light, #eff6ff);
          color: var(--primary-color, #3b82f6);
          border-radius: 8px;
        }

        .resource-info {
          display: flex;
          flex-direction: column;
          gap: 2px;
        }

        .resource-label {
          font-size: 12px;
          color: var(--text-secondary, #6b7280);
        }

        .resource-value {
          font-size: 18px;
          font-weight: 600;
          font-family: monospace;
          color: var(--text-primary, #111827);
        }

        .performance-sections {
          display: flex;
          gap: 4px;
          padding: 0 24px;
          background: var(--surface-color, #ffffff);
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .section-btn {
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

        .section-btn:hover {
          color: var(--text-primary, #111827);
        }

        .section-btn.active {
          color: var(--primary-color, #3b82f6);
          border-bottom-color: var(--primary-color, #3b82f6);
        }

        .performance-content {
          padding: 24px;
          max-width: 1400px;
          margin: 0 auto;
        }

        .performance-section {
          display: flex;
          flex-direction: column;
          gap: 24px;
        }

        .performance-recommendations {
          background: var(--surface-color, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
          padding: 20px;
        }

        .recommendations-title {
          margin: 0 0 16px 0;
          font-size: 16px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .recommendation-card {
          padding: 16px;
          background: var(--surface-secondary, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 6px;
          margin-bottom: 12px;
        }

        .recommendation-card:last-child {
          margin-bottom: 0;
        }

        .recommendation-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 8px;
        }

        .recommendation-key {
          font-size: 14px;
          font-weight: 600;
          font-family: monospace;
          color: var(--text-primary, #111827);
        }

        .recommendation-priority {
          padding: 2px 8px;
          font-size: 11px;
          font-weight: 600;
          text-transform: uppercase;
          border-radius: 4px;
        }

        .recommendation-priority.priority-low {
          background: var(--info-bg, #dbeafe);
          color: var(--info-text, #1e40af);
        }

        .recommendation-priority.priority-medium {
          background: var(--warning-bg, #fef3c7);
          color: var(--warning-text, #92400e);
        }

        .recommendation-priority.priority-high {
          background: var(--error-bg, #fee2e2);
          color: var(--error-text, #991b1b);
        }

        .recommendation-reason {
          margin: 0 0 12px 0;
          font-size: 13px;
          color: var(--text-secondary, #6b7280);
          line-height: 1.5;
        }

        .recommendation-values {
          display: flex;
          align-items: center;
          gap: 12px;
          margin-bottom: 12px;
        }

        .recommendation-value {
          display: flex;
          align-items: center;
          gap: 6px;
        }

        .recommendation-value .label {
          font-size: 11px;
          font-weight: 500;
          color: var(--text-tertiary, #9ca3af);
          text-transform: uppercase;
        }

        .recommendation-value code {
          font-size: 13px;
          font-family: monospace;
          padding: 4px 8px;
          border-radius: 4px;
        }

        .recommendation-value.current code {
          background: var(--error-bg-light, #fef2f2);
          color: var(--error-text, #991b1b);
        }

        .recommendation-value.recommended code {
          background: var(--success-bg-light, #f0fdf4);
          color: var(--success-text, #065f46);
        }

        .recommendation-apply-btn {
          width: 100%;
          padding: 8px 16px;
          font-size: 13px;
          font-weight: 500;
          color: var(--primary-color, #3b82f6);
          background: var(--primary-bg-light, #eff6ff);
          border: 1px solid var(--primary-border, #bfdbfe);
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .recommendation-apply-btn:hover {
          background: var(--primary-bg, #dbeafe);
        }

        /* Dark mode */
        @media (prefers-color-scheme: dark) {
          .performance-config {
            background: var(--background-color, #111827);
          }

          .performance-header,
          .performance-resources,
          .performance-sections {
            background: var(--surface-color, #1f2937);
            border-bottom-color: var(--border-color, #374151);
          }

          .performance-resource-card,
          .performance-recommendations,
          .recommendation-card {
            background: var(--surface-secondary, #111827);
            border-color: var(--border-color, #374151);
          }

          .resource-icon {
            background: var(--primary-bg-light, #1e3a8a);
          }
        }
      `}</style>
    </div>
  );
}

export default PerformanceConfig;
