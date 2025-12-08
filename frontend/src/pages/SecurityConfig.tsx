import React, { useState } from 'react';
import { useConfig } from '../hooks/useConfig';
import { ConfigSection } from '../components/config/ConfigSection';
import { ConfigInput } from '../components/config/ConfigInput';
import { RestartBanner } from '../components/config/RestartBanner';
import { usePendingRestart } from '../hooks/useConfig';

// ============================================================================
// SecurityConfig Page - Security settings
// ============================================================================

export function SecurityConfig() {
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
  } = useConfig({ category: 'security' });

  const { pendingChanges: restartChanges } = usePendingRestart();

  const [activeSection, setActiveSection] = useState<'authentication' | 'ssl' | 'password' | 'session' | 'audit' | 'encryption'>('authentication');

  const handleSaveChanges = async () => {
    await applyChanges();
  };

  // Group settings by subcategory
  const getSettingsForSection = (section: string) => {
    const sectionKeywords: Record<string, string[]> = {
      authentication: ['auth', 'login', 'method'],
      ssl: ['ssl', 'tls', 'cert', 'key'],
      password: ['password', 'pwd'],
      session: ['session', 'timeout', 'idle'],
      audit: ['audit', 'log', 'tracking'],
      encryption: ['encrypt', 'cipher', 'key_rotation'],
    };

    return settings.filter((s) =>
      sectionKeywords[section]?.some((keyword) =>
        s.key.toLowerCase().includes(keyword)
      )
    );
  };

  const getSecurityStatusIcon = () => {
    const authEnabled = getSettingValue('authentication_enabled');
    const sslEnabled = getSettingValue('ssl_enabled');
    const encryptionEnabled = getSettingValue('encryption_enabled');

    const enabledCount = [authEnabled, sslEnabled, encryptionEnabled].filter(Boolean).length;

    if (enabledCount === 3) {
      return {
        icon: 'shield-check',
        color: 'success',
        label: 'High Security',
        message: 'All major security features are enabled',
      };
    } else if (enabledCount >= 1) {
      return {
        icon: 'shield-warning',
        color: 'warning',
        label: 'Medium Security',
        message: 'Some security features are disabled',
      };
    } else {
      return {
        icon: 'shield-alert',
        color: 'error',
        label: 'Low Security',
        message: 'Critical security features are disabled',
      };
    }
  };

  const securityStatus = getSecurityStatusIcon();

  if (loading && settings.length === 0) {
    return (
      <div className="security-config loading">
        <div className="spinner" />
        <p>Loading security configuration...</p>
      </div>
    );
  }

  return (
    <div className="security-config">
      {/* Restart Banner */}
      {restartChanges.length > 0 && (
        <RestartBanner
          pendingChanges={restartChanges}
          onRestartNow={() => console.log('Restart database')}
          onScheduleRestart={() => console.log('Schedule restart')}
        />
      )}

      {/* Page Header */}
      <div className="security-header">
        <div>
          <h1 className="security-title">Security Configuration</h1>
          <p className="security-subtitle">
            Configure authentication, encryption, and access control
          </p>
        </div>

        <div className="security-actions">
          {hasPendingChanges && (
            <>
              <button className="security-action-btn primary" onClick={handleSaveChanges}>
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

              <button className="security-action-btn" onClick={discardChanges}>
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
        <div className="security-error">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
            <path d="M8 4V8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <circle cx="8" cy="11" r="0.5" fill="currentColor" />
          </svg>
          {error}
        </div>
      )}

      {/* Security Status */}
      <div className={`security-status status-${securityStatus.color}`}>
        <div className="security-status-icon">
          <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
            {securityStatus.icon === 'shield-check' && (
              <>
                <path
                  d="M16 4L6 8V14C6 20.5 10 26 16 28C22 26 26 20.5 26 14V8L16 4Z"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinejoin="round"
                />
                <path
                  d="M12 16L15 19L21 13"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </>
            )}
            {securityStatus.icon === 'shield-warning' && (
              <>
                <path
                  d="M16 4L6 8V14C6 20.5 10 26 16 28C22 26 26 20.5 26 14V8L16 4Z"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinejoin="round"
                />
                <path
                  d="M16 12V17"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
                <circle cx="16" cy="21" r="0.75" fill="currentColor" />
              </>
            )}
            {securityStatus.icon === 'shield-alert' && (
              <>
                <path
                  d="M16 4L6 8V14C6 20.5 10 26 16 28C22 26 26 20.5 26 14V8L16 4Z"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinejoin="round"
                />
                <path
                  d="M13 13L19 19M19 13L13 19"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              </>
            )}
          </svg>
        </div>
        <div className="security-status-info">
          <h2 className="security-status-label">{securityStatus.label}</h2>
          <p className="security-status-message">{securityStatus.message}</p>
        </div>
      </div>

      {/* Section Navigation */}
      <div className="security-sections">
        <button
          className={`section-btn ${activeSection === 'authentication' ? 'active' : ''}`}
          onClick={() => setActiveSection('authentication')}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="5" r="3" stroke="currentColor" strokeWidth="1.5" />
            <path
              d="M3 14C3 11.791 5.239 10 8 10C10.761 10 13 11.791 13 14"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
          Authentication
        </button>
        <button
          className={`section-btn ${activeSection === 'ssl' ? 'active' : ''}`}
          onClick={() => setActiveSection('ssl')}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <rect
              x="4"
              y="7"
              width="8"
              height="7"
              rx="1"
              stroke="currentColor"
              strokeWidth="1.5"
            />
            <path
              d="M6 7V5C6 3.895 6.895 3 8 3C9.105 3 10 3.895 10 5V7"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
          SSL/TLS
        </button>
        <button
          className={`section-btn ${activeSection === 'password' ? 'active' : ''}`}
          onClick={() => setActiveSection('password')}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path
              d="M8 3V4M8 12V13M4 8H3M13 8H12M5.5 5.5L4.8 4.8M11.2 11.2L10.5 10.5M10.5 5.5L11.2 4.8M4.8 11.2L5.5 10.5"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
            <circle cx="8" cy="8" r="2" stroke="currentColor" strokeWidth="1.5" />
          </svg>
          Password Policy
        </button>
        <button
          className={`section-btn ${activeSection === 'session' ? 'active' : ''}`}
          onClick={() => setActiveSection('session')}
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
          Session
        </button>
        <button
          className={`section-btn ${activeSection === 'audit' ? 'active' : ''}`}
          onClick={() => setActiveSection('audit')}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <rect
              x="3"
              y="2"
              width="10"
              height="12"
              rx="1"
              stroke="currentColor"
              strokeWidth="1.5"
            />
            <path
              d="M6 6H10M6 9H10"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
          Audit
        </button>
        <button
          className={`section-btn ${activeSection === 'encryption' ? 'active' : ''}`}
          onClick={() => setActiveSection('encryption')}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path
              d="M4 6L8 3L12 6"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
            <path
              d="M4 10L8 13L12 10"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
            <path d="M4 6V10M12 6V10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
          Encryption
        </button>
      </div>

      {/* Content */}
      <div className="security-content">
        {getSettingsForSection(activeSection).length > 0 ? (
          <ConfigSection
            title={activeSection.charAt(0).toUpperCase() + activeSection.slice(1)}
            description={`Configure ${activeSection} settings`}
            category="security"
            settings={getSettingsForSection(activeSection)}
            onResetSection={() => resetCategory('security')}
          >
            {getSettingsForSection(activeSection).map((setting) => (
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
          <div className="security-empty">
            <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
              <path
                d="M24 8L12 14V22C12 31 18 39 24 42C30 39 36 31 36 22V14L24 8Z"
                stroke="currentColor"
                strokeWidth="3"
                strokeLinejoin="round"
              />
            </svg>
            <p>No settings available for this section</p>
          </div>
        )}

        {/* Security Best Practices */}
        <div className="security-best-practices">
          <h3 className="best-practices-title">Security Best Practices</h3>
          <div className="best-practices-grid">
            <div className="best-practice-card">
              <div className="best-practice-icon success">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth="2" />
                  <path
                    d="M7 10L9 12L13 8"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </div>
              <div className="best-practice-content">
                <h4 className="best-practice-title">Enable Authentication</h4>
                <p className="best-practice-description">
                  Always require authentication for database connections
                </p>
              </div>
            </div>

            <div className="best-practice-card">
              <div className="best-practice-icon success">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth="2" />
                  <path
                    d="M7 10L9 12L13 8"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </div>
              <div className="best-practice-content">
                <h4 className="best-practice-title">Use SSL/TLS</h4>
                <p className="best-practice-description">
                  Encrypt all connections using SSL/TLS certificates
                </p>
              </div>
            </div>

            <div className="best-practice-card">
              <div className="best-practice-icon warning">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <path
                    d="M10 2L2 17H18L10 2Z"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinejoin="round"
                  />
                  <path d="M10 7V11" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                  <circle cx="10" cy="14" r="0.5" fill="currentColor" />
                </svg>
              </div>
              <div className="best-practice-content">
                <h4 className="best-practice-title">Strong Passwords</h4>
                <p className="best-practice-description">
                  Enforce minimum length and complexity requirements
                </p>
              </div>
            </div>

            <div className="best-practice-card">
              <div className="best-practice-icon info">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth="2" />
                  <path d="M10 6V10" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                  <circle cx="10" cy="13" r="0.5" fill="currentColor" />
                </svg>
              </div>
              <div className="best-practice-content">
                <h4 className="best-practice-title">Session Timeouts</h4>
                <p className="best-practice-description">
                  Set appropriate timeout values for idle connections
                </p>
              </div>
            </div>

            <div className="best-practice-card">
              <div className="best-practice-icon success">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth="2" />
                  <path
                    d="M7 10L9 12L13 8"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </div>
              <div className="best-practice-content">
                <h4 className="best-practice-title">Enable Audit Logging</h4>
                <p className="best-practice-description">
                  Track all security-related events and access attempts
                </p>
              </div>
            </div>

            <div className="best-practice-card">
              <div className="best-practice-icon warning">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <path
                    d="M10 2L2 17H18L10 2Z"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinejoin="round"
                  />
                  <path d="M10 7V11" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                  <circle cx="10" cy="14" r="0.5" fill="currentColor" />
                </svg>
              </div>
              <div className="best-practice-content">
                <h4 className="best-practice-title">Regular Key Rotation</h4>
                <p className="best-practice-description">
                  Rotate encryption keys periodically for better security
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <style>{`
        .security-config {
          min-height: 100vh;
          background: var(--background-color, #f9fafb);
        }

        .security-config.loading {
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

        .security-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          padding: 24px;
          background: var(--surface-color, #ffffff);
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .security-title {
          margin: 0 0 4px 0;
          font-size: 24px;
          font-weight: 700;
          color: var(--text-primary, #111827);
        }

        .security-subtitle {
          margin: 0;
          font-size: 14px;
          color: var(--text-secondary, #6b7280);
        }

        .security-actions {
          display: flex;
          gap: 8px;
        }

        .security-action-btn {
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

        .security-action-btn.primary {
          background: var(--primary-color, #3b82f6);
          border-color: var(--primary-color, #3b82f6);
          color: #ffffff;
        }

        .security-action-btn.primary:hover {
          background: var(--primary-color-dark, #2563eb);
        }

        .security-action-btn:not(.primary) {
          background: var(--surface-color, #ffffff);
          border-color: var(--border-color, #d1d5db);
          color: var(--text-primary, #111827);
        }

        .security-action-btn:not(.primary):hover {
          background: var(--surface-hover, #f9fafb);
        }

        .security-error {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 24px;
          background: var(--error-bg, #fee2e2);
          color: var(--error-text, #991b1b);
          border-bottom: 1px solid var(--error-border, #fecaca);
        }

        .security-status {
          display: flex;
          align-items: center;
          gap: 16px;
          padding: 24px;
          background: var(--surface-color, #ffffff);
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .security-status.status-success {
          background: var(--success-bg-light, #f0fdf4);
          border-left: 4px solid var(--success-color, #10b981);
        }

        .security-status.status-warning {
          background: var(--warning-bg-light, #fffbeb);
          border-left: 4px solid var(--warning-color, #f59e0b);
        }

        .security-status.status-error {
          background: var(--error-bg-light, #fef2f2);
          border-left: 4px solid var(--error-color, #dc2626);
        }

        .security-status-icon {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 64px;
          height: 64px;
          border-radius: 12px;
        }

        .security-status.status-success .security-status-icon {
          background: var(--success-bg, #d1fae5);
          color: var(--success-color, #10b981);
        }

        .security-status.status-warning .security-status-icon {
          background: var(--warning-bg, #fef3c7);
          color: var(--warning-color, #f59e0b);
        }

        .security-status.status-error .security-status-icon {
          background: var(--error-bg, #fee2e2);
          color: var(--error-color, #dc2626);
        }

        .security-status-info {
          flex: 1;
        }

        .security-status-label {
          margin: 0 0 4px 0;
          font-size: 18px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .security-status-message {
          margin: 0;
          font-size: 14px;
          color: var(--text-secondary, #6b7280);
        }

        .security-sections {
          display: flex;
          gap: 4px;
          padding: 0 24px;
          background: var(--surface-color, #ffffff);
          border-bottom: 1px solid var(--border-color, #e5e7eb);
          overflow-x: auto;
        }

        .section-btn {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 12px 16px;
          font-size: 14px;
          font-weight: 500;
          color: var(--text-secondary, #6b7280);
          background: none;
          border: none;
          border-bottom: 2px solid transparent;
          cursor: pointer;
          transition: all 0.2s;
          white-space: nowrap;
        }

        .section-btn:hover {
          color: var(--text-primary, #111827);
        }

        .section-btn.active {
          color: var(--primary-color, #3b82f6);
          border-bottom-color: var(--primary-color, #3b82f6);
        }

        .security-content {
          padding: 24px;
          max-width: 1400px;
          margin: 0 auto;
        }

        .security-empty {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 64px 24px;
          color: var(--text-secondary, #6b7280);
        }

        .security-empty svg {
          margin-bottom: 16px;
          opacity: 0.5;
        }

        .security-empty p {
          margin: 0;
          font-size: 14px;
        }

        .security-best-practices {
          margin-top: 32px;
          padding: 24px;
          background: var(--surface-color, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
        }

        .best-practices-title {
          margin: 0 0 20px 0;
          font-size: 18px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .best-practices-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
          gap: 16px;
        }

        .best-practice-card {
          display: flex;
          gap: 12px;
          padding: 16px;
          background: var(--surface-secondary, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 6px;
        }

        .best-practice-icon {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 40px;
          height: 40px;
          border-radius: 8px;
          flex-shrink: 0;
        }

        .best-practice-icon.success {
          background: var(--success-bg, #d1fae5);
          color: var(--success-color, #10b981);
        }

        .best-practice-icon.warning {
          background: var(--warning-bg, #fef3c7);
          color: var(--warning-color, #f59e0b);
        }

        .best-practice-icon.info {
          background: var(--info-bg, #dbeafe);
          color: var(--info-color, #3b82f6);
        }

        .best-practice-content {
          flex: 1;
        }

        .best-practice-title {
          margin: 0 0 4px 0;
          font-size: 14px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .best-practice-description {
          margin: 0;
          font-size: 13px;
          color: var(--text-secondary, #6b7280);
          line-height: 1.5;
        }

        /* Dark mode */
        @media (prefers-color-scheme: dark) {
          .security-config {
            background: var(--background-color, #111827);
          }

          .security-header,
          .security-status,
          .security-sections {
            background: var(--surface-color, #1f2937);
            border-bottom-color: var(--border-color, #374151);
          }

          .security-status.status-success {
            background: var(--success-bg-light, #14532d);
          }

          .security-status.status-warning {
            background: var(--warning-bg-light, #78350f);
          }

          .security-status.status-error {
            background: var(--error-bg-light, #7f1d1d);
          }

          .security-best-practices,
          .best-practice-card {
            background: var(--surface-color, #1f2937);
            border-color: var(--border-color, #374151);
          }

          .best-practice-card {
            background: var(--surface-secondary, #111827);
          }
        }
      `}</style>
    </div>
  );
}

export default SecurityConfig;
