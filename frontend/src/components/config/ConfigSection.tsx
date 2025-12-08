import React, { useState } from 'react';
import type { ConfigSetting, ConfigCategory } from '../../services/configService';

// ============================================================================
// ConfigSection Component - Collapsible configuration section
// ============================================================================

interface ConfigSectionProps {
  title: string;
  description?: string;
  category: ConfigCategory;
  settings: ConfigSetting[];
  children: React.ReactNode;
  defaultExpanded?: boolean;
  onResetSection?: () => void;
  showResetButton?: boolean;
}

export function ConfigSection({
  title,
  description,
  category,
  settings,
  children,
  defaultExpanded = true,
  onResetSection,
  showResetButton = true,
}: ConfigSectionProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);

  const modifiedCount = settings.filter((s) => s.isDirty).length;
  const requiresRestartCount = settings.filter((s) => s.isDirty && s.requiresRestart).length;

  const handleResetSection = () => {
    if (onResetSection && window.confirm(`Reset all settings in "${title}" to defaults?`)) {
      onResetSection();
    }
  };

  return (
    <div className="config-section">
      {/* Section Header */}
      <div className="config-section-header">
        <button
          className="config-section-toggle"
          onClick={() => setIsExpanded(!isExpanded)}
          aria-expanded={isExpanded}
          aria-controls={`config-section-${category}`}
        >
          <svg
            className={`config-section-chevron ${isExpanded ? 'expanded' : ''}`}
            width="20"
            height="20"
            viewBox="0 0 20 20"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M7 9L10 12L13 9"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>

          <div className="config-section-title-group">
            <h2 className="config-section-title">{title}</h2>
            {description && (
              <p className="config-section-description">{description}</p>
            )}
          </div>

          {/* Section Badges */}
          <div className="config-section-badges">
            {modifiedCount > 0 && (
              <span className="config-badge config-badge-modified">
                {modifiedCount} modified
              </span>
            )}
            {requiresRestartCount > 0 && (
              <span className="config-badge config-badge-restart">
                {requiresRestartCount} require restart
              </span>
            )}
          </div>
        </button>

        {/* Reset Section Button */}
        {showResetButton && onResetSection && modifiedCount > 0 && (
          <button
            className="config-section-reset-btn"
            onClick={handleResetSection}
            title="Reset all settings in this section to defaults"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 16 16"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
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
            Reset Section
          </button>
        )}
      </div>

      {/* Section Content */}
      {isExpanded && (
        <div
          className="config-section-content"
          id={`config-section-${category}`}
        >
          {children}
        </div>
      )}

      <style>{`
        .config-section {
          background: var(--surface-color, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
          margin-bottom: 16px;
          overflow: hidden;
        }

        .config-section-header {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 16px 20px;
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .config-section-toggle {
          flex: 1;
          display: flex;
          align-items: flex-start;
          gap: 12px;
          padding: 0;
          background: none;
          border: none;
          cursor: pointer;
          text-align: left;
          transition: opacity 0.2s;
        }

        .config-section-toggle:hover {
          opacity: 0.8;
        }

        .config-section-chevron {
          flex-shrink: 0;
          margin-top: 2px;
          color: var(--text-secondary, #6b7280);
          transition: transform 0.2s;
        }

        .config-section-chevron.expanded {
          transform: rotate(0deg);
        }

        .config-section-chevron:not(.expanded) {
          transform: rotate(-90deg);
        }

        .config-section-title-group {
          flex: 1;
        }

        .config-section-title {
          margin: 0;
          font-size: 16px;
          font-weight: 600;
          color: var(--text-primary, #111827);
          line-height: 1.4;
        }

        .config-section-description {
          margin: 4px 0 0 0;
          font-size: 13px;
          color: var(--text-secondary, #6b7280);
          line-height: 1.4;
        }

        .config-section-badges {
          display: flex;
          gap: 8px;
          flex-shrink: 0;
        }

        .config-badge {
          padding: 4px 10px;
          font-size: 12px;
          font-weight: 500;
          border-radius: 12px;
          white-space: nowrap;
        }

        .config-badge-modified {
          background: var(--warning-bg, #fef3c7);
          color: var(--warning-text, #92400e);
        }

        .config-badge-restart {
          background: var(--info-bg, #dbeafe);
          color: var(--info-text, #1e40af);
        }

        .config-section-reset-btn {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 8px 14px;
          font-size: 13px;
          font-weight: 500;
          color: var(--text-secondary, #6b7280);
          background: var(--surface-hover, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.2s;
          flex-shrink: 0;
        }

        .config-section-reset-btn:hover {
          background: var(--surface-active, #f3f4f6);
          color: var(--text-primary, #111827);
        }

        .config-section-reset-btn svg {
          width: 16px;
          height: 16px;
        }

        .config-section-content {
          padding: 0;
        }

        /* Dark mode support */
        @media (prefers-color-scheme: dark) {
          .config-section {
            background: var(--surface-color, #1f2937);
            border-color: var(--border-color, #374151);
          }

          .config-section-header {
            border-bottom-color: var(--border-color, #374151);
          }

          .config-section-title {
            color: var(--text-primary, #f9fafb);
          }

          .config-section-description {
            color: var(--text-secondary, #9ca3af);
          }

          .config-badge-modified {
            background: var(--warning-bg, #78350f);
            color: var(--warning-text, #fcd34d);
          }

          .config-badge-restart {
            background: var(--info-bg, #1e3a8a);
            color: var(--info-text, #93c5fd);
          }

          .config-section-reset-btn {
            color: var(--text-secondary, #9ca3af);
            background: var(--surface-hover, #111827);
            border-color: var(--border-color, #374151);
          }

          .config-section-reset-btn:hover {
            background: var(--surface-active, #1f2937);
            color: var(--text-primary, #f9fafb);
          }
        }
      `}</style>
    </div>
  );
}

export default ConfigSection;
