import React, { useState } from 'react';
import type { ConfigHistoryEntry } from '../../services/configService';

// ============================================================================
// ConfigHistory Component - Timeline of changes with rollback capability
// ============================================================================

interface ConfigHistoryProps {
  history: ConfigHistoryEntry[];
  onRollback?: (historyId: string) => void;
  loading?: boolean;
}

export function ConfigHistory({
  history,
  onRollback,
  loading = false,
}: ConfigHistoryProps) {
  const [expandedEntries, setExpandedEntries] = useState<Set<string>>(new Set());

  const toggleExpanded = (id: string) => {
    setExpandedEntries((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleRollback = (entry: ConfigHistoryEntry) => {
    if (!onRollback) return;

    const confirmMessage = `Are you sure you want to rollback to this configuration?\n\nThis will revert ${entry.changes.length} setting(s) and may require a restart.`;

    if (window.confirm(confirmMessage)) {
      onRollback(entry.id);
    }
  };

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return new Intl.DateTimeFormat('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    }).format(date);
  };

  const formatValue = (value: unknown): string => {
    if (typeof value === 'boolean') return value ? 'Enabled' : 'Disabled';
    if (typeof value === 'number') return value.toLocaleString();
    return String(value);
  };

  const getStatusBadge = (status: ConfigHistoryEntry['status']) => {
    const configs = {
      pending: {
        label: 'Pending',
        className: 'status-pending',
      },
      applied: {
        label: 'Applied',
        className: 'status-applied',
      },
      failed: {
        label: 'Failed',
        className: 'status-failed',
      },
      rolled_back: {
        label: 'Rolled Back',
        className: 'status-rolled-back',
      },
    };

    const config = configs[status];

    return (
      <span className={`config-history-status ${config.className}`}>
        {config.label}
      </span>
    );
  };

  if (loading) {
    return (
      <div className="config-history-loading">
        <div className="spinner" />
        <p>Loading configuration history...</p>
      </div>
    );
  }

  if (history.length === 0) {
    return (
      <div className="config-history-empty">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
          <path
            d="M24 8V24L32 32"
            stroke="currentColor"
            strokeWidth="3"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          <circle cx="24" cy="24" r="18" stroke="currentColor" strokeWidth="3" />
        </svg>
        <p>No configuration history</p>
      </div>
    );
  }

  return (
    <div className="config-history">
      {/* Timeline */}
      <div className="config-history-timeline">
        {history.map((entry, index) => {
          const isExpanded = expandedEntries.has(entry.id);
          const requiresRestartCount = entry.changes.filter((c) => c.requiresRestart).length;

          return (
            <div
              key={entry.id}
              className="config-history-entry"
            >
              {/* Timeline connector */}
              {index < history.length - 1 && (
                <div className="config-history-connector" />
              )}

              {/* Entry Header */}
              <div className="config-history-entry-header">
                <div className="config-history-entry-icon">
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                    <circle cx="8" cy="8" r="3" fill="currentColor" />
                  </svg>
                </div>

                <button
                  className="config-history-entry-info"
                  onClick={() => toggleExpanded(entry.id)}
                >
                  <div className="config-history-entry-title">
                    <span className="config-history-entry-date">
                      {formatDate(entry.timestamp)}
                    </span>
                    {getStatusBadge(entry.status)}
                  </div>

                  <div className="config-history-entry-meta">
                    <span className="config-history-entry-user">
                      {entry.username || 'System'}
                    </span>
                    <span className="config-history-entry-separator">•</span>
                    <span className="config-history-entry-changes">
                      {entry.changes.length} change{entry.changes.length !== 1 ? 's' : ''}
                    </span>
                    {requiresRestartCount > 0 && (
                      <>
                        <span className="config-history-entry-separator">•</span>
                        <span className="config-history-entry-restart">
                          {requiresRestartCount} require restart
                        </span>
                      </>
                    )}
                  </div>

                  {entry.comment && (
                    <p className="config-history-entry-comment">"{entry.comment}"</p>
                  )}
                </button>

                {/* Rollback Button */}
                {onRollback && entry.status === 'applied' && (
                  <button
                    className="config-history-rollback-btn"
                    onClick={() => handleRollback(entry)}
                    title="Rollback to this configuration"
                  >
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
                    Rollback
                  </button>
                )}

                {/* Expand Icon */}
                <button
                  className="config-history-expand-btn"
                  onClick={() => toggleExpanded(entry.id)}
                  aria-label={isExpanded ? 'Collapse' : 'Expand'}
                >
                  <svg
                    width="20"
                    height="20"
                    viewBox="0 0 20 20"
                    fill="none"
                    className={isExpanded ? 'expanded' : ''}
                  >
                    <path
                      d="M7 9L10 12L13 9"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </button>
              </div>

              {/* Expanded Changes */}
              {isExpanded && (
                <div className="config-history-changes">
                  {entry.changes.map((change, changeIndex) => (
                    <div key={changeIndex} className="config-history-change">
                      <div className="config-history-change-header">
                        <code className="config-history-change-key">{change.key}</code>
                        {change.requiresRestart && (
                          <span className="config-history-restart-badge">
                            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                              <path
                                d="M1.5 6C1.5 3.515 3.515 1.5 6 1.5C8.485 1.5 10.5 3.515 10.5 6C10.5 8.485 8.485 10.5 6 10.5C4.682 10.5 3.515 9.849 2.765 8.834"
                                stroke="currentColor"
                                strokeWidth="1.2"
                                strokeLinecap="round"
                              />
                            </svg>
                            Restart
                          </span>
                        )}
                      </div>

                      <div className="config-history-change-values">
                        <div className="config-history-change-value old">
                          <span className="config-history-value-label">From:</span>
                          <code>{formatValue(change.oldValue)}</code>
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
                        <div className="config-history-change-value new">
                          <span className="config-history-value-label">To:</span>
                          <code>{formatValue(change.newValue)}</code>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      <style>{`
        .config-history {
          background: var(--surface-color, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
          padding: 20px;
        }

        .config-history-loading,
        .config-history-empty {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 48px 24px;
          color: var(--text-secondary, #6b7280);
        }

        .config-history-loading .spinner {
          width: 32px;
          height: 32px;
          border: 3px solid var(--border-color, #e5e7eb);
          border-top-color: var(--primary-color, #3b82f6);
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
          margin-bottom: 12px;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }

        .config-history-empty svg {
          margin-bottom: 12px;
          color: var(--text-tertiary, #9ca3af);
        }

        .config-history-empty p {
          margin: 0;
          font-size: 14px;
        }

        .config-history-timeline {
          position: relative;
        }

        .config-history-entry {
          position: relative;
          margin-bottom: 24px;
          padding-left: 40px;
        }

        .config-history-entry:last-child {
          margin-bottom: 0;
        }

        .config-history-entry.selected {
          background: var(--primary-bg-light, #eff6ff);
          border-radius: 6px;
          margin-left: -12px;
          margin-right: -12px;
          padding-left: 52px;
          padding-top: 12px;
          padding-bottom: 12px;
        }

        .config-history-connector {
          position: absolute;
          left: 15px;
          top: 24px;
          bottom: -24px;
          width: 2px;
          background: var(--border-color, #e5e7eb);
        }

        .config-history-entry-icon {
          position: absolute;
          left: 8px;
          top: 4px;
          width: 16px;
          height: 16px;
          color: var(--primary-color, #3b82f6);
        }

        .config-history-entry-header {
          display: flex;
          align-items: flex-start;
          gap: 12px;
        }

        .config-history-entry-info {
          flex: 1;
          padding: 0;
          background: none;
          border: none;
          text-align: left;
          cursor: pointer;
        }

        .config-history-entry-title {
          display: flex;
          align-items: center;
          gap: 10px;
          margin-bottom: 4px;
        }

        .config-history-entry-date {
          font-size: 15px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .config-history-status {
          padding: 2px 8px;
          font-size: 11px;
          font-weight: 600;
          text-transform: uppercase;
          border-radius: 4px;
        }

        .config-history-status.status-applied {
          background: var(--success-bg, #d1fae5);
          color: var(--success-text, #065f46);
        }

        .config-history-status.status-pending {
          background: var(--info-bg, #dbeafe);
          color: var(--info-text, #1e40af);
        }

        .config-history-status.status-failed {
          background: var(--error-bg, #fee2e2);
          color: var(--error-text, #991b1b);
        }

        .config-history-status.status-rolled-back {
          background: var(--warning-bg, #fef3c7);
          color: var(--warning-text, #92400e);
        }

        .config-history-entry-meta {
          display: flex;
          align-items: center;
          gap: 6px;
          font-size: 13px;
          color: var(--text-secondary, #6b7280);
          margin-bottom: 4px;
        }

        .config-history-entry-user {
          font-weight: 500;
        }

        .config-history-entry-separator {
          color: var(--text-tertiary, #d1d5db);
        }

        .config-history-entry-restart {
          color: var(--info-text, #1e40af);
          font-weight: 500;
        }

        .config-history-entry-comment {
          margin: 4px 0 0 0;
          font-size: 13px;
          font-style: italic;
          color: var(--text-secondary, #6b7280);
        }

        .config-history-rollback-btn {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 6px 12px;
          font-size: 13px;
          font-weight: 500;
          color: var(--primary-color, #3b82f6);
          background: var(--primary-bg-light, #eff6ff);
          border: 1px solid var(--primary-border, #bfdbfe);
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.2s;
          flex-shrink: 0;
        }

        .config-history-rollback-btn:hover {
          background: var(--primary-bg, #dbeafe);
        }

        .config-history-expand-btn {
          padding: 4px;
          background: none;
          border: none;
          color: var(--text-secondary, #6b7280);
          cursor: pointer;
          transition: color 0.2s;
          flex-shrink: 0;
        }

        .config-history-expand-btn:hover {
          color: var(--text-primary, #111827);
        }

        .config-history-expand-btn svg {
          transition: transform 0.2s;
        }

        .config-history-expand-btn svg.expanded {
          transform: rotate(180deg);
        }

        .config-history-changes {
          margin-top: 12px;
          padding: 12px;
          background: var(--surface-secondary, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 6px;
        }

        .config-history-change {
          padding: 10px 0;
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .config-history-change:last-child {
          border-bottom: none;
          padding-bottom: 0;
        }

        .config-history-change:first-child {
          padding-top: 0;
        }

        .config-history-change-header {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-bottom: 8px;
        }

        .config-history-change-key {
          font-size: 13px;
          font-weight: 600;
          font-family: monospace;
          color: var(--text-primary, #111827);
        }

        .config-history-restart-badge {
          display: flex;
          align-items: center;
          gap: 3px;
          padding: 2px 6px;
          font-size: 10px;
          font-weight: 600;
          text-transform: uppercase;
          background: var(--info-bg, #dbeafe);
          color: var(--info-text, #1e40af);
          border-radius: 4px;
        }

        .config-history-change-values {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .config-history-change-value {
          display: flex;
          align-items: center;
          gap: 6px;
        }

        .config-history-value-label {
          font-size: 11px;
          font-weight: 500;
          color: var(--text-tertiary, #9ca3af);
          text-transform: uppercase;
        }

        .config-history-change-value code {
          font-size: 13px;
          font-family: monospace;
          padding: 4px 8px;
          border-radius: 4px;
        }

        .config-history-change-value.old code {
          background: var(--error-bg-light, #fef2f2);
          color: var(--error-text, #991b1b);
        }

        .config-history-change-value.new code {
          background: var(--success-bg-light, #f0fdf4);
          color: var(--success-text, #065f46);
        }

        .config-history-change-values > svg {
          color: var(--text-tertiary, #9ca3af);
          flex-shrink: 0;
        }

        /* Dark mode */
        @media (prefers-color-scheme: dark) {
          .config-history {
            background: var(--surface-color, #1f2937);
            border-color: var(--border-color, #374151);
          }

          .config-history-entry.selected {
            background: var(--primary-bg-light, #1e3a8a);
          }

          .config-history-changes {
            background: var(--surface-secondary, #111827);
            border-color: var(--border-color, #374151);
          }

          .config-history-change {
            border-bottom-color: var(--border-color, #374151);
          }

          .config-history-status.status-applied {
            background: var(--success-bg, #14532d);
            color: var(--success-text, #86efac);
          }

          .config-history-status.status-pending {
            background: var(--info-bg, #1e3a8a);
            color: var(--info-text, #93c5fd);
          }

          .config-history-status.status-failed {
            background: var(--error-bg, #7f1d1d);
            color: var(--error-text, #fca5a5);
          }

          .config-history-status.status-rolled-back {
            background: var(--warning-bg, #78350f);
            color: var(--warning-text, #fcd34d);
          }

          .config-history-change-value.old code {
            background: var(--error-bg-light, #7f1d1d);
            color: var(--error-text, #fca5a5);
          }

          .config-history-change-value.new code {
            background: var(--success-bg-light, #14532d);
            color: var(--success-text, #86efac);
          }
        }
      `}</style>
    </div>
  );
}

export default ConfigHistory;
