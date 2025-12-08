import React, { useState } from 'react';
import type { ConfigChange } from '../../services/configService';

// ============================================================================
// RestartBanner Component - Notification banner for restart requirements
// ============================================================================

interface RestartBannerProps {
  pendingChanges: ConfigChange[];
  onRestartNow?: () => void;
  onScheduleRestart?: () => void;
  onDismiss?: () => void;
  showScheduleOption?: boolean;
}

export function RestartBanner({
  pendingChanges,
  onRestartNow,
  onScheduleRestart,
  onDismiss,
  showScheduleOption = true,
}: RestartBannerProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [isDismissed, setIsDismissed] = useState(false);

  if (pendingChanges.length === 0 || isDismissed) {
    return null;
  }

  const handleDismiss = () => {
    setIsDismissed(true);
    onDismiss?.();
  };

  const handleRestartNow = () => {
    if (window.confirm('Are you sure you want to restart the database now? This will temporarily interrupt all connections.')) {
      onRestartNow?.();
    }
  };

  const formatValue = (value: unknown): string => {
    if (typeof value === 'boolean') return value ? 'Enabled' : 'Disabled';
    if (typeof value === 'number') return value.toLocaleString();
    return String(value);
  };

  return (
    <div className="restart-banner">
      <div className="restart-banner-content">
        {/* Icon */}
        <div className="restart-banner-icon">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <path
              d="M3 12C3 7.029 7.029 3 12 3C16.971 3 21 7.029 21 12C21 16.971 16.971 21 12 21C9.364 21 7.03 19.698 5.529 17.667"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
            />
            <path
              d="M3 16.5L3 12L7.5 12"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>

        {/* Main Message */}
        <div className="restart-banner-message">
          <h3 className="restart-banner-title">
            Restart Required
          </h3>
          <p className="restart-banner-description">
            {pendingChanges.length} configuration change{pendingChanges.length !== 1 ? 's' : ''} require{pendingChanges.length === 1 ? 's' : ''} a database restart to take effect.
          </p>

          {/* Expandable Changes List */}
          {isExpanded && (
            <div className="restart-banner-changes">
              <h4 className="restart-banner-changes-title">Pending Changes:</h4>
              <ul className="restart-banner-changes-list">
                {pendingChanges.map((change, index) => (
                  <li key={index} className="restart-banner-change-item">
                    <code className="restart-banner-change-key">{change.key}</code>
                    <span className="restart-banner-change-arrow">→</span>
                    <span className="restart-banner-change-value">
                      {formatValue(change.newValue)}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* Toggle Button */}
          <button
            className="restart-banner-toggle"
            onClick={() => setIsExpanded(!isExpanded)}
          >
            {isExpanded ? 'Hide' : 'View'} changes
            <svg
              width="16"
              height="16"
              viewBox="0 0 16 16"
              fill="none"
              className={isExpanded ? 'expanded' : ''}
            >
              <path
                d="M6 7L8 9L10 7"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
        </div>

        {/* Actions */}
        <div className="restart-banner-actions">
          {showScheduleOption && onScheduleRestart && (
            <button
              className="restart-banner-btn secondary"
              onClick={onScheduleRestart}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="1.5" />
                <path
                  d="M8 4V8L11 9"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                />
              </svg>
              Schedule Restart
            </button>
          )}

          {onRestartNow && (
            <button
              className="restart-banner-btn primary"
              onClick={handleRestartNow}
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
              Restart Now
            </button>
          )}

          {onDismiss && (
            <button
              className="restart-banner-close"
              onClick={handleDismiss}
              title="Dismiss"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path
                  d="M4 4L12 12M12 4L4 12"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                />
              </svg>
            </button>
          )}
        </div>
      </div>

      <style>{`
        .restart-banner {
          position: sticky;
          top: 0;
          z-index: 40;
          background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
          color: #ffffff;
          box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -1px rgb(0 0 0 / 0.06);
          animation: slideDown 0.3s ease-out;
        }

        @keyframes slideDown {
          from {
            transform: translateY(-100%);
            opacity: 0;
          }
          to {
            transform: translateY(0);
            opacity: 1;
          }
        }

        .restart-banner-content {
          display: flex;
          align-items: flex-start;
          gap: 16px;
          padding: 16px 24px;
          max-width: 1400px;
          margin: 0 auto;
        }

        .restart-banner-icon {
          flex-shrink: 0;
          display: flex;
          align-items: center;
          justify-content: center;
          width: 40px;
          height: 40px;
          background: rgba(255, 255, 255, 0.2);
          border-radius: 8px;
        }

        .restart-banner-message {
          flex: 1;
        }

        .restart-banner-title {
          margin: 0 0 4px 0;
          font-size: 16px;
          font-weight: 600;
          color: #ffffff;
        }

        .restart-banner-description {
          margin: 0 0 8px 0;
          font-size: 14px;
          color: rgba(255, 255, 255, 0.9);
          line-height: 1.5;
        }

        .restart-banner-changes {
          margin-top: 12px;
          padding: 12px;
          background: rgba(0, 0, 0, 0.15);
          border-radius: 6px;
        }

        .restart-banner-changes-title {
          margin: 0 0 8px 0;
          font-size: 13px;
          font-weight: 600;
          color: #ffffff;
        }

        .restart-banner-changes-list {
          margin: 0;
          padding: 0;
          list-style: none;
        }

        .restart-banner-change-item {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 6px 0;
          font-size: 13px;
          color: rgba(255, 255, 255, 0.95);
        }

        .restart-banner-change-key {
          font-family: monospace;
          font-weight: 600;
          background: rgba(0, 0, 0, 0.2);
          padding: 2px 6px;
          border-radius: 3px;
        }

        .restart-banner-change-arrow {
          color: rgba(255, 255, 255, 0.6);
        }

        .restart-banner-change-value {
          font-family: monospace;
          background: rgba(255, 255, 255, 0.15);
          padding: 2px 6px;
          border-radius: 3px;
        }

        .restart-banner-toggle {
          display: inline-flex;
          align-items: center;
          gap: 4px;
          padding: 4px 8px;
          margin-top: 4px;
          font-size: 13px;
          font-weight: 500;
          color: #ffffff;
          background: rgba(255, 255, 255, 0.15);
          border: 1px solid rgba(255, 255, 255, 0.2);
          border-radius: 4px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .restart-banner-toggle:hover {
          background: rgba(255, 255, 255, 0.25);
        }

        .restart-banner-toggle svg {
          transition: transform 0.2s;
        }

        .restart-banner-toggle svg.expanded {
          transform: rotate(180deg);
        }

        .restart-banner-actions {
          display: flex;
          align-items: center;
          gap: 8px;
          flex-shrink: 0;
        }

        .restart-banner-btn {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 10px 18px;
          font-size: 14px;
          font-weight: 500;
          border-radius: 6px;
          border: 1px solid;
          cursor: pointer;
          transition: all 0.2s;
          white-space: nowrap;
        }

        .restart-banner-btn.primary {
          background: #ffffff;
          border-color: #ffffff;
          color: #3b82f6;
        }

        .restart-banner-btn.primary:hover {
          background: #f3f4f6;
          border-color: #f3f4f6;
        }

        .restart-banner-btn.secondary {
          background: rgba(255, 255, 255, 0.15);
          border-color: rgba(255, 255, 255, 0.3);
          color: #ffffff;
        }

        .restart-banner-btn.secondary:hover {
          background: rgba(255, 255, 255, 0.25);
        }

        .restart-banner-close {
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 8px;
          background: none;
          border: none;
          color: rgba(255, 255, 255, 0.8);
          cursor: pointer;
          transition: color 0.2s;
        }

        .restart-banner-close:hover {
          color: #ffffff;
        }

        /* Responsive Design */
        @media (max-width: 768px) {
          .restart-banner-content {
            flex-direction: column;
          }

          .restart-banner-actions {
            width: 100%;
            justify-content: stretch;
          }

          .restart-banner-btn {
            flex: 1;
            justify-content: center;
          }

          .restart-banner-close {
            position: absolute;
            top: 16px;
            right: 16px;
          }
        }

        /* Print styles */
        @media print {
          .restart-banner {
            display: none;
          }
        }
      `}</style>
    </div>
  );
}

export default RestartBanner;
