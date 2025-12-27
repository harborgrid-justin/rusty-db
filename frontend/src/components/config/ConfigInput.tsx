import React, { useState } from 'react';
import type { ConfigSetting } from '../../services/configService';

// ============================================================================
// ConfigInput Component - Type-appropriate input for configuration settings
// ============================================================================

interface ConfigInputProps {
  setting: ConfigSetting;
  value: unknown;
  onChange: (value: unknown) => void;
  onReset?: () => void;
  error?: string;
  isDirty?: boolean;
  disabled?: boolean;
  showResetButton?: boolean;
}

export function ConfigInput({
  setting,
  value,
  onChange,
  onReset,
  error,
  isDirty = false,
  disabled = false,
  showResetButton = true,
}: ConfigInputProps) {
  const [showTooltip, setShowTooltip] = useState(false);

  const isDifferentFromDefault = value !== setting.defaultValue;

  const renderInput = () => {
    switch (setting.dataType) {
      case 'boolean':
        return (
          <label className="config-input-toggle">
            <input
              type="checkbox"
              checked={!!value}
              onChange={(e) => onChange(e.target.checked)}
              disabled={disabled}
              className="config-toggle-input"
            />
            <span className="config-toggle-slider" />
            <span className="config-toggle-label">
              {value ? 'Enabled' : 'Disabled'}
            </span>
          </label>
        );

      case 'enum':
        return (
          <select
            value={String(value)}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            className="config-select"
          >
            {setting.allowedValues?.map((val) => (
              <option key={String(val)} value={String(val)}>
                {String(val)}
              </option>
            ))}
          </select>
        );

      case 'number':
        return (
          <div className="config-number-input-group">
            <input
              type="number"
              value={Number(value)}
              onChange={(e) => onChange(Number(e.target.value))}
              min={setting.minValue}
              max={setting.maxValue}
              disabled={disabled}
              className="config-number-input"
            />
            {setting.unit && (
              <span className="config-input-unit">{setting.unit}</span>
            )}
          </div>
        );

      default:
        return (
          <input
            type="text"
            value={String(value)}
            onChange={(e) => onChange(e.target.value)}
            disabled={disabled}
            className="config-text-input"
          />
        );
    }
  };

  const renderValueComparison = () => {
    if (!isDifferentFromDefault && !isDirty) return null;

    return (
      <div className="config-value-comparison">
        {isDirty && (
          <div className="config-value-item">
            <span className="config-value-label">Current:</span>
            <span className="config-value-text">{String(setting.currentValue)}</span>
          </div>
        )}
        {isDifferentFromDefault && (
          <div className="config-value-item">
            <span className="config-value-label">Default:</span>
            <span className="config-value-text">{String(setting.defaultValue)}</span>
          </div>
        )}
      </div>
    );
  };

  return (
    <div className={`config-input-wrapper ${error ? 'has-error' : ''} ${isDirty ? 'is-dirty' : ''}`}>
      {/* Input Header */}
      <div className="config-input-header">
        <div className="config-input-label-group">
          <label className="config-input-label">
            {setting.key}
            {setting.requiresRestart && (
              <span className="config-restart-badge" title="Requires restart">
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                  <path
                    d="M1.5 6C1.5 3.515 3.515 1.5 6 1.5C8.485 1.5 10.5 3.515 10.5 6C10.5 8.485 8.485 10.5 6 10.5C4.682 10.5 3.515 9.849 2.765 8.834"
                    stroke="currentColor"
                    strokeWidth="1.2"
                    strokeLinecap="round"
                  />
                </svg>
              </span>
            )}
          </label>

          {/* Help Tooltip */}
          <button
            type="button"
            className="config-help-btn"
            onMouseEnter={() => setShowTooltip(true)}
            onMouseLeave={() => setShowTooltip(false)}
            aria-label="Help"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <circle cx="7" cy="7" r="6" stroke="currentColor" strokeWidth="1.2" />
              <path
                d="M5.5 5.5C5.5 4.672 6.172 4 7 4C7.828 4 8.5 4.672 8.5 5.5C8.5 6.328 7.828 7 7 7V8"
                stroke="currentColor"
                strokeWidth="1.2"
                strokeLinecap="round"
              />
              <circle cx="7" cy="10" r="0.5" fill="currentColor" />
            </svg>
          </button>

          {/* Tooltip */}
          {showTooltip && (
            <div className="config-tooltip">
              <div className="config-tooltip-content">
                <p className="config-tooltip-description">{setting.description}</p>
                {setting.minValue !== undefined && setting.maxValue !== undefined && (
                  <p className="config-tooltip-range">
                    Range: {setting.minValue} - {setting.maxValue}
                    {setting.unit && ` ${setting.unit}`}
                  </p>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Reset Button */}
        {showResetButton && onReset && isDirty && (
          <button
            type="button"
            className="config-reset-btn"
            onClick={onReset}
            title="Reset to current saved value"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path
                d="M2 7C2 4.515 4.015 2.5 6.5 2.5C8.985 2.5 11 4.515 11 7C11 9.485 8.985 11.5 6.5 11.5C5.182 11.5 4.015 10.849 3.265 9.834"
                stroke="currentColor"
                strokeWidth="1.2"
                strokeLinecap="round"
              />
              <path
                d="M2 9.5L2 7L4.5 7"
                stroke="currentColor"
                strokeWidth="1.2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
        )}
      </div>

      {/* Input Control */}
      <div className="config-input-control">{renderInput()}</div>

      {/* Value Comparison */}
      {renderValueComparison()}

      {/* Error Message */}
      {error && (
        <div className="config-input-error">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <circle cx="7" cy="7" r="6" stroke="currentColor" strokeWidth="1.2" />
            <path
              d="M7 4V7"
              stroke="currentColor"
              strokeWidth="1.2"
              strokeLinecap="round"
            />
            <circle cx="7" cy="9.5" r="0.5" fill="currentColor" />
          </svg>
          {error}
        </div>
      )}

      <style>{`
        .config-input-wrapper {
          padding: 16px;
          border-bottom: 1px solid var(--border-color, #e5e7eb);
          transition: background-color 0.2s;
        }

        .config-input-wrapper:hover {
          background-color: var(--surface-hover, #f9fafb);
        }

        .config-input-wrapper.has-error {
          background-color: var(--error-bg-light, #fef2f2);
        }

        .config-input-wrapper.is-dirty {
          background-color: var(--warning-bg-light, #fffbeb);
        }

        .config-input-wrapper:last-child {
          border-bottom: none;
        }

        .config-input-header {
          display: flex;
          align-items: flex-start;
          justify-content: space-between;
          margin-bottom: 8px;
        }

        .config-input-label-group {
          display: flex;
          align-items: center;
          gap: 6px;
          position: relative;
        }

        .config-input-label {
          font-size: 13px;
          font-weight: 500;
          color: var(--text-primary, #111827);
          display: flex;
          align-items: center;
          gap: 4px;
        }

        .config-restart-badge {
          display: inline-flex;
          align-items: center;
          color: var(--info-color, #3b82f6);
        }

        .config-help-btn {
          padding: 0;
          background: none;
          border: none;
          color: var(--text-secondary, #6b7280);
          cursor: pointer;
          display: flex;
          align-items: center;
          transition: color 0.2s;
        }

        .config-help-btn:hover {
          color: var(--text-primary, #111827);
        }

        .config-tooltip {
          position: absolute;
          top: 100%;
          left: 0;
          margin-top: 4px;
          z-index: 50;
        }

        .config-tooltip-content {
          background: var(--surface-elevated, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 6px;
          padding: 12px;
          box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);
          max-width: 300px;
        }

        .config-tooltip-description {
          margin: 0;
          font-size: 12px;
          color: var(--text-secondary, #6b7280);
          line-height: 1.5;
        }

        .config-tooltip-range {
          margin: 6px 0 0 0;
          font-size: 11px;
          font-weight: 500;
          color: var(--text-tertiary, #9ca3af);
        }

        .config-reset-btn {
          padding: 4px;
          background: none;
          border: none;
          color: var(--text-secondary, #6b7280);
          cursor: pointer;
          display: flex;
          align-items: center;
          transition: color 0.2s;
        }

        .config-reset-btn:hover {
          color: var(--primary-color, #3b82f6);
        }

        .config-input-control {
          margin-bottom: 8px;
        }

        /* Text Input */
        .config-text-input {
          width: 100%;
          padding: 8px 12px;
          font-size: 14px;
          color: var(--text-primary, #111827);
          background: var(--input-bg, #ffffff);
          border: 1px solid var(--border-color, #d1d5db);
          border-radius: 6px;
          transition: all 0.2s;
        }

        .config-text-input:focus {
          outline: none;
          border-color: var(--primary-color, #3b82f6);
          box-shadow: 0 0 0 3px var(--primary-color-alpha, rgba(59, 130, 246, 0.1));
        }

        .config-text-input:disabled {
          opacity: 0.6;
          cursor: not-allowed;
        }

        /* Number Input */
        .config-number-input-group {
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .config-number-input {
          width: 200px;
          padding: 8px 12px;
          font-size: 14px;
          color: var(--text-primary, #111827);
          background: var(--input-bg, #ffffff);
          border: 1px solid var(--border-color, #d1d5db);
          border-radius: 6px;
          transition: all 0.2s;
        }

        .config-number-input:focus {
          outline: none;
          border-color: var(--primary-color, #3b82f6);
          box-shadow: 0 0 0 3px var(--primary-color-alpha, rgba(59, 130, 246, 0.1));
        }

        .config-input-unit {
          font-size: 13px;
          font-weight: 500;
          color: var(--text-secondary, #6b7280);
        }

        /* Select */
        .config-select {
          width: 100%;
          max-width: 300px;
          padding: 8px 32px 8px 12px;
          font-size: 14px;
          color: var(--text-primary, #111827);
          background: var(--input-bg, #ffffff);
          border: 1px solid var(--border-color, #d1d5db);
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.2s;
          appearance: none;
          background-image: url("data:image/svg+xml,%3Csvg width='12' height='8' viewBox='0 0 12 8' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1L6 6L11 1' stroke='%236B7280' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
          background-repeat: no-repeat;
          background-position: right 12px center;
        }

        .config-select:focus {
          outline: none;
          border-color: var(--primary-color, #3b82f6);
          box-shadow: 0 0 0 3px var(--primary-color-alpha, rgba(59, 130, 246, 0.1));
        }

        /* Toggle */
        .config-input-toggle {
          display: flex;
          align-items: center;
          gap: 12px;
          cursor: pointer;
        }

        .config-toggle-input {
          position: absolute;
          opacity: 0;
          pointer-events: none;
        }

        .config-toggle-slider {
          position: relative;
          display: inline-block;
          width: 44px;
          height: 24px;
          background-color: var(--toggle-bg-off, #d1d5db);
          border-radius: 12px;
          transition: background-color 0.2s;
        }

        .config-toggle-slider::after {
          content: '';
          position: absolute;
          top: 2px;
          left: 2px;
          width: 20px;
          height: 20px;
          background-color: #ffffff;
          border-radius: 50%;
          transition: transform 0.2s;
        }

        .config-toggle-input:checked + .config-toggle-slider {
          background-color: var(--primary-color, #3b82f6);
        }

        .config-toggle-input:checked + .config-toggle-slider::after {
          transform: translateX(20px);
        }

        .config-toggle-input:disabled + .config-toggle-slider {
          opacity: 0.6;
          cursor: not-allowed;
        }

        .config-toggle-label {
          font-size: 14px;
          font-weight: 500;
          color: var(--text-secondary, #6b7280);
        }

        /* Value Comparison */
        .config-value-comparison {
          display: flex;
          gap: 16px;
          margin-bottom: 8px;
        }

        .config-value-item {
          display: flex;
          align-items: center;
          gap: 6px;
          font-size: 12px;
        }

        .config-value-label {
          font-weight: 500;
          color: var(--text-tertiary, #9ca3af);
        }

        .config-value-text {
          font-family: monospace;
          color: var(--text-secondary, #6b7280);
          background: var(--code-bg, #f3f4f6);
          padding: 2px 6px;
          border-radius: 3px;
        }

        /* Error */
        .config-input-error {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 8px 12px;
          font-size: 12px;
          color: var(--error-text, #dc2626);
          background: var(--error-bg, #fee2e2);
          border-radius: 6px;
        }

        /* Dark mode */
        @media (prefers-color-scheme: dark) {
          .config-input-wrapper:hover {
            background-color: var(--surface-hover, #111827);
          }

          .config-input-wrapper.has-error {
            background-color: var(--error-bg-light, #7f1d1d);
          }

          .config-input-wrapper.is-dirty {
            background-color: var(--warning-bg-light, #78350f);
          }

          .config-tooltip-content {
            background: var(--surface-elevated, #1f2937);
            border-color: var(--border-color, #374151);
            box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.4);
          }

          .config-text-input,
          .config-number-input,
          .config-select {
            background: var(--input-bg, #111827);
            border-color: var(--border-color, #374151);
            color: var(--text-primary, #f9fafb);
          }

          .config-value-text {
            background: var(--code-bg, #111827);
          }

          .config-input-error {
            background: var(--error-bg, #7f1d1d);
            color: var(--error-text, #fca5a5);
          }
        }
      `}</style>
    </div>
  );
}

export default ConfigInput;
