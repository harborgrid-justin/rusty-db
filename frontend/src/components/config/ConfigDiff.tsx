import React from 'react';
import type { ConfigSetting } from '../../services/configService';

// ============================================================================
// ConfigDiff Component - Before/after comparison with impact warnings
// ============================================================================

interface ConfigDiffProps {
  changes: Array<{
    setting: ConfigSetting;
    oldValue: unknown;
    newValue: unknown;
  }>;
  onApply?: () => void;
  onCancel?: () => void;
  showActions?: boolean;
}

export function ConfigDiff({
  changes,
  onApply,
  onCancel,
  showActions = true,
}: ConfigDiffProps) {
  if (changes.length === 0) {
    return (
      <div className="config-diff-empty">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
          <circle cx="24" cy="24" r="20" stroke="currentColor" strokeWidth="2" />
          <path d="M24 14V24L30 30" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <p>No configuration changes</p>
      </div>
    );
  }

  const requiresRestartChanges = changes.filter((c) => c.setting.requiresRestart);
  const immediateChanges = changes.filter((c) => !c.setting.requiresRestart);

  const formatValue = (value: unknown): string => {
    if (typeof value === 'boolean') return value ? 'Enabled' : 'Disabled';
    if (typeof value === 'number') return value.toLocaleString();
    return String(value);
  };

  const getImpactLevel = (setting: ConfigSetting): 'low' | 'medium' | 'high' => {
    if (setting.requiresRestart) return 'high';
    if (setting.category === 'security' || setting.category === 'performance') return 'medium';
    return 'low';
  };

  const getImpactBadge = (impact: 'low' | 'medium' | 'high') => {
    const labels = {
      low: 'Low Impact',
      medium: 'Medium Impact',
      high: 'High Impact',
    };

    return (
      <span className={`config-diff-impact-badge impact-${impact}`}>
        {labels[impact]}
      </span>
    );
  };

  return (
    <div className="config-diff">
      {/* Header */}
      <div className="config-diff-header">
        <div>
          <h3 className="config-diff-title">Configuration Changes</h3>
          <p className="config-diff-subtitle">
            Review {changes.length} pending change{changes.length !== 1 ? 's' : ''}
          </p>
        </div>

        {requiresRestartChanges.length > 0 && (
          <div className="config-diff-restart-notice">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path
                d="M2 8C2 4.686 4.686 2 8 2C11.314 2 14 4.686 14 8C14 11.314 11.314 14 8 14C6.243 14 4.687 13.132 3.686 11.778"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
              <path
                d="M2 11L2 8L5 8"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            <span>
              {requiresRestartChanges.length} change{requiresRestartChanges.length !== 1 ? 's' : ''} require restart
            </span>
          </div>
        )}
      </div>

      {/* Changes List */}
      <div className="config-diff-list">
        {changes.map((change, index) => {
          const impact = getImpactLevel(change.setting);
          const hasChanged = change.oldValue !== change.newValue;

          return (
            <div key={index} className={`config-diff-item impact-${impact}`}>
              {/* Setting Header */}
              <div className="config-diff-item-header">
                <div className="config-diff-item-title">
                  <span className="config-diff-setting-key">{change.setting.key}</span>
                  <span className="config-diff-category">{change.setting.category}</span>
                </div>
                <div className="config-diff-item-badges">
                  {getImpactBadge(impact)}
                  {change.setting.requiresRestart && (
                    <span className="config-diff-restart-badge" title="Requires restart">
                      <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                        <path
                          d="M1.5 6C1.5 3.515 3.515 1.5 6 1.5C8.485 1.5 10.5 3.515 10.5 6C10.5 8.485 8.485 10.5 6 10.5C4.682 10.5 3.515 9.849 2.765 8.834"
                          stroke="currentColor"
                          strokeWidth="1.2"
                          strokeLinecap="round"
                        />
                      </svg>
                      Restart Required
                    </span>
                  )}
                </div>
              </div>

              {/* Description */}
              {change.setting.description && (
                <p className="config-diff-description">{change.setting.description}</p>
              )}

              {/* Value Comparison */}
              <div className="config-diff-comparison">
                <div className="config-diff-value old-value">
                  <span className="config-diff-value-label">Current</span>
                  <div className="config-diff-value-box">
                    <code>{formatValue(change.oldValue)}</code>
                    {change.setting.unit && (
                      <span className="config-diff-unit">{change.setting.unit}</span>
                    )}
                  </div>
                </div>

                <div className="config-diff-arrow">
                  <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                    <path
                      d="M5 10H15M15 10L11 6M15 10L11 14"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </div>

                <div className="config-diff-value new-value">
                  <span className="config-diff-value-label">New</span>
                  <div className="config-diff-value-box">
                    <code>{formatValue(change.newValue)}</code>
                    {change.setting.unit && (
                      <span className="config-diff-unit">{change.setting.unit}</span>
                    )}
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Summary */}
      <div className="config-diff-summary">
        <div className="config-diff-summary-item">
          <span className="config-diff-summary-label">Total Changes:</span>
          <span className="config-diff-summary-value">{changes.length}</span>
        </div>
        {immediateChanges.length > 0 && (
          <div className="config-diff-summary-item">
            <span className="config-diff-summary-label">Immediate:</span>
            <span className="config-diff-summary-value">{immediateChanges.length}</span>
          </div>
        )}
        {requiresRestartChanges.length > 0 && (
          <div className="config-diff-summary-item">
            <span className="config-diff-summary-label">Requires Restart:</span>
            <span className="config-diff-summary-value">{requiresRestartChanges.length}</span>
          </div>
        )}
      </div>

      {/* Actions */}
      {showActions && (onApply || onCancel) && (
        <div className="config-diff-actions">
          {onCancel && (
            <button
              type="button"
              className="config-diff-btn config-diff-btn-cancel"
              onClick={onCancel}
            >
              Cancel
            </button>
          )}
          {onApply && (
            <button
              type="button"
              className="config-diff-btn config-diff-btn-apply"
              onClick={onApply}
            >
              Apply Changes
              {requiresRestartChanges.length > 0 && (
                <span className="config-diff-btn-notice">
                  (Restart Required)
                </span>
              )}
            </button>
          )}
        </div>
      )}

      <style>{`
        .config-diff {
          background: var(--surface-color, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
          overflow: hidden;
        }

        .config-diff-empty {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 48px 24px;
          color: var(--text-secondary, #6b7280);
        }

        .config-diff-empty svg {
          margin-bottom: 12px;
          opacity: 0.5;
        }

        .config-diff-empty p {
          margin: 0;
          font-size: 14px;
        }

        .config-diff-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          padding: 20px;
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .config-diff-title {
          margin: 0 0 4px 0;
          font-size: 18px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .config-diff-subtitle {
          margin: 0;
          font-size: 14px;
          color: var(--text-secondary, #6b7280);
        }

        .config-diff-restart-notice {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 8px 12px;
          background: var(--info-bg, #dbeafe);
          color: var(--info-text, #1e40af);
          border-radius: 6px;
          font-size: 13px;
          font-weight: 500;
        }

        .config-diff-list {
          max-height: 500px;
          overflow-y: auto;
        }

        .config-diff-item {
          padding: 20px;
          border-bottom: 1px solid var(--border-color, #e5e7eb);
          border-left: 3px solid transparent;
        }

        .config-diff-item.impact-low {
          border-left-color: var(--success-color, #10b981);
        }

        .config-diff-item.impact-medium {
          border-left-color: var(--warning-color, #f59e0b);
        }

        .config-diff-item.impact-high {
          border-left-color: var(--error-color, #dc2626);
        }

        .config-diff-item:last-child {
          border-bottom: none;
        }

        .config-diff-item-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 8px;
        }

        .config-diff-item-title {
          display: flex;
          align-items: center;
          gap: 10px;
        }

        .config-diff-setting-key {
          font-size: 15px;
          font-weight: 600;
          font-family: monospace;
          color: var(--text-primary, #111827);
        }

        .config-diff-category {
          padding: 2px 8px;
          font-size: 11px;
          font-weight: 500;
          text-transform: uppercase;
          background: var(--surface-secondary, #f3f4f6);
          color: var(--text-secondary, #6b7280);
          border-radius: 4px;
        }

        .config-diff-item-badges {
          display: flex;
          gap: 8px;
        }

        .config-diff-impact-badge {
          padding: 4px 10px;
          font-size: 12px;
          font-weight: 500;
          border-radius: 12px;
        }

        .config-diff-impact-badge.impact-low {
          background: var(--success-bg, #d1fae5);
          color: var(--success-text, #065f46);
        }

        .config-diff-impact-badge.impact-medium {
          background: var(--warning-bg, #fef3c7);
          color: var(--warning-text, #92400e);
        }

        .config-diff-impact-badge.impact-high {
          background: var(--error-bg, #fee2e2);
          color: var(--error-text, #991b1b);
        }

        .config-diff-restart-badge {
          display: flex;
          align-items: center;
          gap: 4px;
          padding: 4px 10px;
          font-size: 12px;
          font-weight: 500;
          background: var(--info-bg, #dbeafe);
          color: var(--info-text, #1e40af);
          border-radius: 12px;
        }

        .config-diff-description {
          margin: 0 0 12px 0;
          font-size: 13px;
          color: var(--text-secondary, #6b7280);
          line-height: 1.5;
        }

        .config-diff-comparison {
          display: flex;
          align-items: center;
          gap: 16px;
        }

        .config-diff-value {
          flex: 1;
        }

        .config-diff-value-label {
          display: block;
          margin-bottom: 6px;
          font-size: 12px;
          font-weight: 500;
          color: var(--text-tertiary, #9ca3af);
          text-transform: uppercase;
        }

        .config-diff-value-box {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 14px;
          background: var(--surface-secondary, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 6px;
        }

        .config-diff-value.old-value .config-diff-value-box {
          background: var(--error-bg-light, #fef2f2);
          border-color: var(--error-border-light, #fecaca);
        }

        .config-diff-value.new-value .config-diff-value-box {
          background: var(--success-bg-light, #f0fdf4);
          border-color: var(--success-border-light, #bbf7d0);
        }

        .config-diff-value-box code {
          font-family: monospace;
          font-size: 14px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .config-diff-unit {
          font-size: 12px;
          color: var(--text-secondary, #6b7280);
        }

        .config-diff-arrow {
          color: var(--text-tertiary, #9ca3af);
          flex-shrink: 0;
        }

        .config-diff-summary {
          display: flex;
          gap: 24px;
          padding: 16px 20px;
          background: var(--surface-secondary, #f9fafb);
          border-top: 1px solid var(--border-color, #e5e7eb);
        }

        .config-diff-summary-item {
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .config-diff-summary-label {
          font-size: 13px;
          color: var(--text-secondary, #6b7280);
        }

        .config-diff-summary-value {
          font-size: 14px;
          font-weight: 600;
          font-family: monospace;
          color: var(--text-primary, #111827);
        }

        .config-diff-actions {
          display: flex;
          gap: 12px;
          padding: 16px 20px;
          border-top: 1px solid var(--border-color, #e5e7eb);
          justify-content: flex-end;
        }

        .config-diff-btn {
          padding: 10px 20px;
          font-size: 14px;
          font-weight: 500;
          border-radius: 6px;
          border: 1px solid;
          cursor: pointer;
          transition: all 0.2s;
        }

        .config-diff-btn-cancel {
          background: var(--surface-color, #ffffff);
          border-color: var(--border-color, #d1d5db);
          color: var(--text-primary, #111827);
        }

        .config-diff-btn-cancel:hover {
          background: var(--surface-hover, #f9fafb);
        }

        .config-diff-btn-apply {
          background: var(--primary-color, #3b82f6);
          border-color: var(--primary-color, #3b82f6);
          color: #ffffff;
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .config-diff-btn-apply:hover {
          background: var(--primary-color-dark, #2563eb);
        }

        .config-diff-btn-notice {
          font-size: 12px;
          opacity: 0.9;
        }

        /* Dark mode */
        @media (prefers-color-scheme: dark) {
          .config-diff {
            background: var(--surface-color, #1f2937);
            border-color: var(--border-color, #374151);
          }

          .config-diff-header,
          .config-diff-summary,
          .config-diff-actions {
            border-color: var(--border-color, #374151);
          }

          .config-diff-item {
            border-bottom-color: var(--border-color, #374151);
          }

          .config-diff-restart-notice {
            background: var(--info-bg, #1e3a8a);
            color: var(--info-text, #93c5fd);
          }

          .config-diff-value.old-value .config-diff-value-box {
            background: var(--error-bg-light, #7f1d1d);
            border-color: var(--error-border-light, #991b1b);
          }

          .config-diff-value.new-value .config-diff-value-box {
            background: var(--success-bg-light, #14532d);
            border-color: var(--success-border-light, #166534);
          }

          .config-diff-summary {
            background: var(--surface-secondary, #111827);
          }
        }
      `}</style>
    </div>
  );
}

export default ConfigDiff;
