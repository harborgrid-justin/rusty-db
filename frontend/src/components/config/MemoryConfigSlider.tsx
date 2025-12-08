import React, { useMemo } from 'react';

// ============================================================================
// MemoryConfigSlider Component - Visual memory allocation with warnings
// ============================================================================

interface MemoryAllocation {
  name: string;
  key: string;
  value: number;
  color: string;
  description: string;
}

interface MemoryConfigSliderProps {
  totalSystemMemory: number; // in bytes
  allocations: MemoryAllocation[];
  onChange: (key: string, value: number) => void;
  showWarnings?: boolean;
}

export function MemoryConfigSlider({
  totalSystemMemory,
  allocations,
  onChange,
  showWarnings = true,
}: MemoryConfigSliderProps) {
  // Calculate total allocated memory
  const totalAllocated = useMemo(() => {
    return allocations.reduce((sum, alloc) => sum + alloc.value, 0);
  }, [allocations]);

  // Calculate percentages
  const allocationPercent = (totalAllocated / totalSystemMemory) * 100;
  const isOverAllocated = allocationPercent > 90;
  const isNearLimit = allocationPercent > 75 && allocationPercent <= 90;

  // Format bytes to human-readable
  const formatBytes = (bytes: number): string => {
    const gb = bytes / (1024 * 1024 * 1024);
    if (gb >= 1) return `${gb.toFixed(2)} GB`;
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(0)} MB`;
  };

  // Calculate percentage for each allocation
  const getAllocationPercent = (bytes: number): number => {
    return (bytes / totalSystemMemory) * 100;
  };

  // Get recommended range for a setting
  const getRecommendedRange = (key: string): { min: number; max: number } => {
    switch (key) {
      case 'shared_buffers':
        return {
          min: totalSystemMemory * 0.15, // 15%
          max: totalSystemMemory * 0.40, // 40%
        };
      case 'effective_cache_size':
        return {
          min: totalSystemMemory * 0.50, // 50%
          max: totalSystemMemory * 0.75, // 75%
        };
      case 'work_mem':
        return {
          min: 4 * 1024 * 1024, // 4 MB
          max: 128 * 1024 * 1024, // 128 MB
        };
      case 'maintenance_work_mem':
        return {
          min: 64 * 1024 * 1024, // 64 MB
          max: 2 * 1024 * 1024 * 1024, // 2 GB
        };
      default:
        return { min: 0, max: totalSystemMemory };
    }
  };

  return (
    <div className="memory-config-slider">
      {/* System Memory Header */}
      <div className="memory-header">
        <div className="memory-header-info">
          <h3 className="memory-title">Memory Allocation</h3>
          <p className="memory-subtitle">
            Total System Memory: <strong>{formatBytes(totalSystemMemory)}</strong>
          </p>
        </div>
        <div className="memory-stats">
          <div className="memory-stat">
            <span className="memory-stat-label">Allocated:</span>
            <span className={`memory-stat-value ${isOverAllocated ? 'over-allocated' : isNearLimit ? 'near-limit' : ''}`}>
              {formatBytes(totalAllocated)} ({allocationPercent.toFixed(1)}%)
            </span>
          </div>
          <div className="memory-stat">
            <span className="memory-stat-label">Available:</span>
            <span className="memory-stat-value">
              {formatBytes(totalSystemMemory - totalAllocated)}
            </span>
          </div>
        </div>
      </div>

      {/* Visual Memory Bar */}
      <div className="memory-visual-bar">
        <div className="memory-bar-track">
          {allocations.map((alloc, index) => {
            const percent = getAllocationPercent(alloc.value);
            const left = allocations
              .slice(0, index)
              .reduce((sum, a) => sum + getAllocationPercent(a.value), 0);

            return (
              <div
                key={alloc.key}
                className="memory-bar-segment"
                style={{
                  left: `${left}%`,
                  width: `${percent}%`,
                  backgroundColor: alloc.color,
                }}
                title={`${alloc.name}: ${formatBytes(alloc.value)}`}
              />
            );
          })}
        </div>

        {/* Warning markers */}
        <div className="memory-bar-markers">
          <div className="memory-marker" style={{ left: '75%' }}>
            <div className="memory-marker-line warning" />
            <span className="memory-marker-label">75%</span>
          </div>
          <div className="memory-marker" style={{ left: '90%' }}>
            <div className="memory-marker-line danger" />
            <span className="memory-marker-label">90%</span>
          </div>
        </div>
      </div>

      {/* Warnings */}
      {showWarnings && isOverAllocated && (
        <div className="memory-warning danger">
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
          <strong>Warning:</strong> Total memory allocation exceeds 90% of system memory.
          This may cause performance issues or system instability.
        </div>
      )}

      {showWarnings && isNearLimit && !isOverAllocated && (
        <div className="memory-warning caution">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
            <path d="M8 4V8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <circle cx="8" cy="11" r="0.5" fill="currentColor" />
          </svg>
          <strong>Caution:</strong> Memory allocation is approaching system limits.
          Monitor performance and adjust if needed.
        </div>
      )}

      {/* Individual Memory Settings */}
      <div className="memory-settings-list">
        {allocations.map((alloc) => {
          const recommended = getRecommendedRange(alloc.key);
          const isInRecommendedRange = alloc.value >= recommended.min && alloc.value <= recommended.max;
          const percent = getAllocationPercent(alloc.value);

          return (
            <div key={alloc.key} className="memory-setting-item">
              <div className="memory-setting-header">
                <div className="memory-setting-label">
                  <div
                    className="memory-setting-color-dot"
                    style={{ backgroundColor: alloc.color }}
                  />
                  <div>
                    <span className="memory-setting-name">{alloc.name}</span>
                    <span className="memory-setting-description">{alloc.description}</span>
                  </div>
                </div>
                <div className="memory-setting-value">
                  {formatBytes(alloc.value)}
                  <span className="memory-setting-percent">({percent.toFixed(1)}%)</span>
                </div>
              </div>

              <div className="memory-slider-container">
                <input
                  type="range"
                  min={0}
                  max={totalSystemMemory}
                  step={64 * 1024 * 1024} // 64 MB steps
                  value={alloc.value}
                  onChange={(e) => onChange(alloc.key, Number(e.target.value))}
                  className={`memory-slider ${!isInRecommendedRange ? 'out-of-range' : ''}`}
                  style={{
                    background: `linear-gradient(to right, ${alloc.color} 0%, ${alloc.color} ${percent}%, #e5e7eb ${percent}%, #e5e7eb 100%)`,
                  }}
                />
                <div className="memory-slider-labels">
                  <span className="memory-slider-label">0</span>
                  <span className="memory-slider-label recommended-marker">
                    Recommended: {formatBytes(recommended.min)} - {formatBytes(recommended.max)}
                  </span>
                  <span className="memory-slider-label">{formatBytes(totalSystemMemory)}</span>
                </div>
              </div>

              {!isInRecommendedRange && (
                <div className="memory-setting-recommendation">
                  Recommended range: {formatBytes(recommended.min)} - {formatBytes(recommended.max)}
                </div>
              )}
            </div>
          );
        })}
      </div>

      <style>{`
        .memory-config-slider {
          padding: 20px;
          background: var(--surface-color, #ffffff);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 8px;
        }

        .memory-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 20px;
          padding-bottom: 16px;
          border-bottom: 1px solid var(--border-color, #e5e7eb);
        }

        .memory-title {
          margin: 0 0 4px 0;
          font-size: 16px;
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .memory-subtitle {
          margin: 0;
          font-size: 13px;
          color: var(--text-secondary, #6b7280);
        }

        .memory-subtitle strong {
          font-weight: 600;
          color: var(--text-primary, #111827);
        }

        .memory-stats {
          display: flex;
          gap: 24px;
        }

        .memory-stat {
          display: flex;
          flex-direction: column;
          align-items: flex-end;
          gap: 2px;
        }

        .memory-stat-label {
          font-size: 12px;
          color: var(--text-tertiary, #9ca3af);
        }

        .memory-stat-value {
          font-size: 14px;
          font-weight: 600;
          font-family: monospace;
          color: var(--text-primary, #111827);
        }

        .memory-stat-value.near-limit {
          color: var(--warning-color, #f59e0b);
        }

        .memory-stat-value.over-allocated {
          color: var(--error-color, #dc2626);
        }

        .memory-visual-bar {
          position: relative;
          margin-bottom: 24px;
        }

        .memory-bar-track {
          position: relative;
          height: 32px;
          background: var(--surface-secondary, #f3f4f6);
          border-radius: 6px;
          overflow: hidden;
        }

        .memory-bar-segment {
          position: absolute;
          top: 0;
          height: 100%;
          transition: all 0.3s ease;
          border-right: 1px solid rgba(255, 255, 255, 0.3);
        }

        .memory-bar-markers {
          position: relative;
          height: 24px;
        }

        .memory-marker {
          position: absolute;
          top: 0;
          transform: translateX(-50%);
        }

        .memory-marker-line {
          width: 2px;
          height: 20px;
        }

        .memory-marker-line.warning {
          background: var(--warning-color, #f59e0b);
        }

        .memory-marker-line.danger {
          background: var(--error-color, #dc2626);
        }

        .memory-marker-label {
          position: absolute;
          top: 22px;
          left: 50%;
          transform: translateX(-50%);
          font-size: 11px;
          font-weight: 500;
          color: var(--text-tertiary, #9ca3af);
          white-space: nowrap;
        }

        .memory-warning {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 16px;
          border-radius: 6px;
          font-size: 13px;
          line-height: 1.5;
          margin-bottom: 20px;
        }

        .memory-warning.danger {
          background: var(--error-bg, #fee2e2);
          color: var(--error-text, #991b1b);
          border: 1px solid var(--error-border, #fecaca);
        }

        .memory-warning.caution {
          background: var(--warning-bg, #fef3c7);
          color: var(--warning-text, #92400e);
          border: 1px solid var(--warning-border, #fde68a);
        }

        .memory-warning svg {
          flex-shrink: 0;
        }

        .memory-settings-list {
          display: flex;
          flex-direction: column;
          gap: 24px;
        }

        .memory-setting-item {
          padding: 16px;
          background: var(--surface-secondary, #f9fafb);
          border: 1px solid var(--border-color, #e5e7eb);
          border-radius: 6px;
        }

        .memory-setting-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 12px;
        }

        .memory-setting-label {
          display: flex;
          align-items: flex-start;
          gap: 10px;
        }

        .memory-setting-color-dot {
          width: 12px;
          height: 12px;
          border-radius: 50%;
          margin-top: 3px;
          flex-shrink: 0;
        }

        .memory-setting-name {
          display: block;
          font-size: 14px;
          font-weight: 600;
          color: var(--text-primary, #111827);
          margin-bottom: 2px;
        }

        .memory-setting-description {
          display: block;
          font-size: 12px;
          color: var(--text-secondary, #6b7280);
        }

        .memory-setting-value {
          font-size: 14px;
          font-weight: 600;
          font-family: monospace;
          color: var(--text-primary, #111827);
          display: flex;
          align-items: baseline;
          gap: 6px;
        }

        .memory-setting-percent {
          font-size: 12px;
          font-weight: 500;
          color: var(--text-secondary, #6b7280);
        }

        .memory-slider-container {
          margin-bottom: 8px;
        }

        .memory-slider {
          width: 100%;
          height: 8px;
          border-radius: 4px;
          outline: none;
          cursor: pointer;
          appearance: none;
          -webkit-appearance: none;
        }

        .memory-slider::-webkit-slider-thumb {
          appearance: none;
          -webkit-appearance: none;
          width: 18px;
          height: 18px;
          background: var(--primary-color, #3b82f6);
          border: 2px solid #ffffff;
          border-radius: 50%;
          cursor: pointer;
          box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }

        .memory-slider::-moz-range-thumb {
          width: 18px;
          height: 18px;
          background: var(--primary-color, #3b82f6);
          border: 2px solid #ffffff;
          border-radius: 50%;
          cursor: pointer;
          box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }

        .memory-slider.out-of-range::-webkit-slider-thumb {
          background: var(--warning-color, #f59e0b);
        }

        .memory-slider.out-of-range::-moz-range-thumb {
          background: var(--warning-color, #f59e0b);
        }

        .memory-slider-labels {
          display: flex;
          justify-content: space-between;
          margin-top: 6px;
        }

        .memory-slider-label {
          font-size: 11px;
          color: var(--text-tertiary, #9ca3af);
        }

        .memory-slider-label.recommended-marker {
          color: var(--text-secondary, #6b7280);
          font-weight: 500;
        }

        .memory-setting-recommendation {
          font-size: 12px;
          color: var(--warning-text, #92400e);
          background: var(--warning-bg-light, #fffbeb);
          padding: 6px 10px;
          border-radius: 4px;
          border: 1px solid var(--warning-border, #fde68a);
        }

        /* Dark mode */
        @media (prefers-color-scheme: dark) {
          .memory-config-slider {
            background: var(--surface-color, #1f2937);
            border-color: var(--border-color, #374151);
          }

          .memory-header {
            border-bottom-color: var(--border-color, #374151);
          }

          .memory-bar-track {
            background: var(--surface-secondary, #111827);
          }

          .memory-setting-item {
            background: var(--surface-secondary, #111827);
            border-color: var(--border-color, #374151);
          }

          .memory-warning.danger {
            background: var(--error-bg, #7f1d1d);
            color: var(--error-text, #fca5a5);
            border-color: var(--error-border, #991b1b);
          }

          .memory-warning.caution {
            background: var(--warning-bg, #78350f);
            color: var(--warning-text, #fcd34d);
            border-color: var(--warning-border, #92400e);
          }

          .memory-setting-recommendation {
            background: var(--warning-bg-light, #78350f);
            color: var(--warning-text, #fcd34d);
            border-color: var(--warning-border, #92400e);
          }
        }
      `}</style>
    </div>
  );
}

export default MemoryConfigSlider;
