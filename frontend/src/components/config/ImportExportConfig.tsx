import React, { useState, useRef } from 'react';
import type { ConfigExport, ConfigValidationResult } from '../../services/configService';

// ============================================================================
// ImportExportConfig Component - Import/Export configuration with validation
// ============================================================================

interface ImportExportConfigProps {
  onExport?: (includeDefaults: boolean) => Promise<void>;
  onImport?: (config: ConfigExport) => Promise<ConfigValidationResult | null>;
  exportLoading?: boolean;
  importLoading?: boolean;
}

export function ImportExportConfig({
  onExport,
  onImport,
  exportLoading = false,
  importLoading = false,
}: ImportExportConfigProps) {
  const [includeDefaults, setIncludeDefaults] = useState(false);
  const [importPreview, setImportPreview] = useState<ConfigExport | null>(null);
  const [validationResult, setValidationResult] = useState<ConfigValidationResult | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleExport = async () => {
    if (!onExport) return;
    await onExport(includeDefaults);
  };

  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const content = e.target?.result as string;
        const config = JSON.parse(content) as ConfigExport;

        // Basic validation
        if (!config.version || !config.settings) {
          setImportError('Invalid configuration file format');
          return;
        }

        setImportPreview(config);
        setImportError(null);
      } catch (error) {
        setImportError('Failed to parse configuration file');
        setImportPreview(null);
      }
    };
    reader.readAsText(file);
  };

  const handleImport = async () => {
    if (!importPreview || !onImport) return;

    const result = await onImport(importPreview);
    setValidationResult(result);

    if (result && result.valid) {
      // Clear preview after successful import
      setImportPreview(null);
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    }
  };

  const handleCancelImport = () => {
    setImportPreview(null);
    setValidationResult(null);
    setImportError(null);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
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

  return (
    <div className="import-export-config">
      {/* Export Section */}
      <div className="import-export-section">
        <div className="import-export-header">
          <div>
            <h3 className="import-export-title">Export Configuration</h3>
            <p className="import-export-description">
              Download current configuration as a JSON file
            </p>
          </div>
          <svg width="40" height="40" viewBox="0 0 40 40" fill="none">
            <rect
              x="8"
              y="12"
              width="24"
              height="20"
              rx="2"
              stroke="currentColor"
              strokeWidth="2"
            />
            <path
              d="M20 8V18M20 18L16 14M20 18L24 14"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>

        <div className="import-export-options">
          <label className="import-export-checkbox">
            <input
              type="checkbox"
              checked={includeDefaults}
              onChange={(e) => setIncludeDefaults(e.target.checked)}
            />
            <span>Include default values</span>
          </label>
        </div>

        <button
          className="import-export-btn primary"
          onClick={handleExport}
          disabled={exportLoading}
        >
          {exportLoading ? (
            <>
              <div className="btn-spinner" />
              Exporting...
            </>
          ) : (
            <>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path
                  d="M8 2V10M8 10L5 7M8 10L11 7"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
                <path
                  d="M2 14H14"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                />
              </svg>
              Export Configuration
            </>
          )}
        </button>
      </div>

      <div className="import-export-divider" />

      {/* Import Section */}
      <div className="import-export-section">
        <div className="import-export-header">
          <div>
            <h3 className="import-export-title">Import Configuration</h3>
            <p className="import-export-description">
              Upload a configuration file to apply settings
            </p>
          </div>
          <svg width="40" height="40" viewBox="0 0 40 40" fill="none">
            <rect
              x="8"
              y="12"
              width="24"
              height="20"
              rx="2"
              stroke="currentColor"
              strokeWidth="2"
            />
            <path
              d="M20 22V14M20 14L16 18M20 14L24 18"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>

        {!importPreview ? (
          <>
            <input
              ref={fileInputRef}
              type="file"
              accept=".json"
              onChange={handleFileSelect}
              className="import-export-file-input"
              id="config-file-input"
            />
            <label htmlFor="config-file-input" className="import-export-file-label">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                <path
                  d="M7 10L12 15L17 10"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
                <path
                  d="M12 15V3"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
                <rect
                  x="4"
                  y="17"
                  width="16"
                  height="4"
                  rx="1"
                  stroke="currentColor"
                  strokeWidth="2"
                />
              </svg>
              <span className="import-export-file-text">
                Click to browse or drag and drop
              </span>
              <span className="import-export-file-hint">JSON files only</span>
            </label>

            {importError && (
              <div className="import-export-error">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
                  <path
                    d="M8 4V8"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                  />
                  <circle cx="8" cy="11" r="0.5" fill="currentColor" />
                </svg>
                {importError}
              </div>
            )}
          </>
        ) : (
          <div className="import-export-preview">
            <div className="import-export-preview-header">
              <h4 className="import-export-preview-title">Configuration Preview</h4>
              <button
                className="import-export-preview-close"
                onClick={handleCancelImport}
                title="Cancel import"
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
            </div>

            <div className="import-export-preview-details">
              <div className="import-export-preview-detail">
                <span className="label">Version:</span>
                <span className="value">{importPreview.version}</span>
              </div>
              <div className="import-export-preview-detail">
                <span className="label">Exported:</span>
                <span className="value">{formatDate(importPreview.exportedAt)}</span>
              </div>
              <div className="import-export-preview-detail">
                <span className="label">Settings:</span>
                <span className="value">
                  {Object.keys(importPreview.settings).length} configuration{Object.keys(importPreview.settings).length !== 1 ? 's' : ''}
                </span>
              </div>
            </div>

            {validationResult && (
              <div className="import-export-validation">
                {validationResult.errors.length > 0 && (
                  <div className="import-export-validation-errors">
                    <h5 className="import-export-validation-title error">
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
                        <path d="M8 4V8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                        <circle cx="8" cy="11" r="0.5" fill="currentColor" />
                      </svg>
                      {validationResult.errors.length} Error{validationResult.errors.length !== 1 ? 's' : ''}
                    </h5>
                    <ul className="import-export-validation-list">
                      {validationResult.errors.map((error, index) => (
                        <li key={index}>
                          <code>{error.key}:</code> {error.message}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {validationResult.warnings.length > 0 && (
                  <div className="import-export-validation-warnings">
                    <h5 className="import-export-validation-title warning">
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <path
                          d="M8 1L1 14H15L8 1Z"
                          stroke="currentColor"
                          strokeWidth="1.5"
                          strokeLinejoin="round"
                        />
                        <path d="M8 6V9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                        <circle cx="8" cy="11.5" r="0.5" fill="currentColor" />
                      </svg>
                      {validationResult.warnings.length} Warning{validationResult.warnings.length !== 1 ? 's' : ''}
                    </h5>
                    <ul className="import-export-validation-list">
                      {validationResult.warnings.map((warning, index) => (
                        <li key={index}>
                          <code>{warning.key}:</code> {warning.message}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {validationResult.valid && validationResult.errors.length === 0 && (
                  <div className="import-export-validation-success">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                      <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
                      <path
                        d="M5 8L7 10L11 6"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                    Configuration is valid and ready to import
                  </div>
                )}
              </div>
            )}

            <div className="import-export-preview-actions">
              <button
                className="import-export-btn secondary"
                onClick={handleCancelImport}
              >
                Cancel
              </button>
              <button
                className="import-export-btn primary"
                onClick={handleImport}
                disabled={importLoading || (validationResult?.errors.length ?? 0) > 0}
              >
                {importLoading ? (
                  <>
                    <div className="btn-spinner" />
                    Importing...
                  </>
                ) : (
                  <>
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                      <path
                        d="M8 14V6M8 6L5 9M8 6L11 9"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                      <path
                        d="M2 2H14"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                      />
                    </svg>
                    Apply Configuration
                  </>
                )}
              </button>
            </div>
          </div>
        )}
      </div>

      <style>{`
        .import-export-config {
          background: var(--surface-color, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
          overflow: hidden;
        }

        .import-export-section {
          padding: 24px;
        }

        .import-export-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 20px;
        }

        .import-export-header svg {
          color: var(--text-tertiary, #9ca3af);
        }

        .import-export-title {
          margin: 0 0 4px 0;
          font-size: 16px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .import-export-description {
          margin: 0;
          font-size: 13px;
          color: var(--text-secondary, #6b7280);
        }

        .import-export-divider {
          height: 1px;
          background: var(--border-color, #e5e7eb);
        }

        .import-export-options {
          margin-bottom: 16px;
        }

        .import-export-checkbox {
          display: flex;
          align-items: center;
          gap: 8px;
          cursor: pointer;
          font-size: 14px;
          color: var(--text-primary, #111827);
        }

        .import-export-checkbox input[type="checkbox"] {
          width: 18px;
          height: 18px;
          cursor: pointer;
        }

        .import-export-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
          width: 100%;
          padding: 12px 20px;
          font-size: 14px;
          font-weight: 500;
          border-radius: 6px;
          border: 1px solid;
          cursor: pointer;
          transition: all 0.2s;
        }

        .import-export-btn:disabled {
          opacity: 0.6;
          cursor: not-allowed;
        }

        .import-export-btn.primary {
          background: var(--primary-color, #3b82f6);
          border-color: var(--primary-color, #3b82f6);
          color: #ffffff;
        }

        .import-export-btn.primary:hover:not(:disabled) {
          background: var(--primary-color-dark, #2563eb);
        }

        .import-export-btn.secondary {
          background: var(--surface-color, #ffffff);
          border-color: var(--border-color, #d1d5db);
          color: var(--text-primary, #111827);
        }

        .import-export-btn.secondary:hover:not(:disabled) {
          background: var(--surface-hover, #f9fafb);
        }

        .btn-spinner {
          width: 14px;
          height: 14px;
          border: 2px solid rgba(255, 255, 255, 0.3);
          border-top-color: #ffffff;
          border-radius: 50%;
          animation: spin 0.6s linear infinite;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }

        .import-export-file-input {
          display: none;
        }

        .import-export-file-label {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 48px 24px;
          border: 2px dashed var(--border-color, #d1d5db);
          border-radius: 8px;
          background: var(--surface-secondary, #f9fafb);
          cursor: pointer;
          transition: all 0.2s;
        }

        .import-export-file-label:hover {
          border-color: var(--primary-color, #3b82f6);
          background: var(--primary-bg-light, #eff6ff);
        }

        .import-export-file-label svg {
          margin-bottom: 12px;
          color: var(--text-tertiary, #9ca3af);
        }

        .import-export-file-text {
          display: block;
          font-size: 14px;
          font-weight: 500;
          color: var(--text-primary, #111827);
          margin-bottom: 4px;
        }

        .import-export-file-hint {
          display: block;
          font-size: 12px;
          color: var(--text-secondary, #6b7280);
        }

        .import-export-error {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 16px;
          margin-top: 16px;
          background: var(--error-bg, #fee2e2);
          color: var(--error-text, #991b1b);
          border: 1px solid var(--error-border, #fecaca);
          border-radius: 6px;
          font-size: 13px;
        }

        .import-export-preview {
          padding: 20px;
          background: var(--surface-secondary, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
        }

        .import-export-preview-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 16px;
        }

        .import-export-preview-title {
          margin: 0;
          font-size: 15px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .import-export-preview-close {
          padding: 4px;
          background: none;
          border: none;
          color: var(--text-secondary, #6b7280);
          cursor: pointer;
          transition: color 0.2s;
        }

        .import-export-preview-close:hover {
          color: var(--text-primary, #111827);
        }

        .import-export-preview-details {
          display: flex;
          flex-direction: column;
          gap: 8px;
          margin-bottom: 16px;
        }

        .import-export-preview-detail {
          display: flex;
          align-items: center;
          font-size: 13px;
        }

        .import-export-preview-detail .label {
          font-weight: 500;
          color: var(--text-secondary, #6b7280);
          width: 100px;
        }

        .import-export-preview-detail .value {
          color: var(--text-primary, #111827);
          font-family: monospace;
        }

        .import-export-validation {
          margin-bottom: 16px;
        }

        .import-export-validation-errors,
        .import-export-validation-warnings {
          margin-bottom: 12px;
          padding: 12px;
          border-radius: 6px;
        }

        .import-export-validation-errors {
          background: var(--error-bg, #fee2e2);
          border: 1px solid var(--error-border, #fecaca);
        }

        .import-export-validation-warnings {
          background: var(--warning-bg, #fef3c7);
          border: 1px solid var(--warning-border, #fde68a);
        }

        .import-export-validation-title {
          display: flex;
          align-items: center;
          gap: 6px;
          margin: 0 0 8px 0;
          font-size: 13px;
          font-weight: 600;
        }

        .import-export-validation-title.error {
          color: var(--error-text, #991b1b);
        }

        .import-export-validation-title.warning {
          color: var(--warning-text, #92400e);
        }

        .import-export-validation-list {
          margin: 0;
          padding-left: 24px;
          font-size: 12px;
        }

        .import-export-validation-list li {
          margin-bottom: 4px;
          line-height: 1.5;
        }

        .import-export-validation-list code {
          font-weight: 600;
        }

        .import-export-validation-success {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px;
          background: var(--success-bg, #d1fae5);
          color: var(--success-text, #065f46);
          border: 1px solid var(--success-border, #a7f3d0);
          border-radius: 6px;
          font-size: 13px;
          font-weight: 500;
        }

        .import-export-preview-actions {
          display: flex;
          gap: 12px;
        }

        .import-export-preview-actions .import-export-btn {
          width: auto;
          flex: 1;
        }

        /* Dark mode */
        @media (prefers-color-scheme: dark) {
          .import-export-config {
            background: var(--surface-color, #1f2937);
            border-color: var(--border-color, #374151);
          }

          .import-export-file-label {
            background: var(--surface-secondary, #111827);
            border-color: var(--border-color, #374151);
          }

          .import-export-file-label:hover {
            background: var(--primary-bg-light, #1e3a8a);
            border-color: var(--primary-color, #3b82f6);
          }

          .import-export-preview {
            background: var(--surface-secondary, #111827);
            border-color: var(--border-color, #374151);
          }

          .import-export-error {
            background: var(--error-bg, #7f1d1d);
            color: var(--error-text, #fca5a5);
            border-color: var(--error-border, #991b1b);
          }

          .import-export-validation-errors {
            background: var(--error-bg, #7f1d1d);
            border-color: var(--error-border, #991b1b);
          }

          .import-export-validation-errors .import-export-validation-title {
            color: var(--error-text, #fca5a5);
          }

          .import-export-validation-warnings {
            background: var(--warning-bg, #78350f);
            border-color: var(--warning-border, #92400e);
          }

          .import-export-validation-warnings .import-export-validation-title {
            color: var(--warning-text, #fcd34d);
          }

          .import-export-validation-success {
            background: var(--success-bg, #14532d);
            color: var(--success-text, #86efac);
            border-color: var(--success-border, #166534);
          }
        }
      `}</style>
    </div>
  );
}

export default ImportExportConfig;
