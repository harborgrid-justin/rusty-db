// ============================================================================
// Security Event Timeline Component
// Visualizes security events over time
// ============================================================================

import { useMemo } from 'react';
import { motion } from 'framer-motion';
import { ChartBarIcon } from '@heroicons/react/24/outline';
import clsx from 'clsx';

// ============================================================================
// Component Props
// ============================================================================

interface SecurityEventTimelineProps {
  timeline: Array<{
    timestamp: string;
    count: number;
  }>;
}

// ============================================================================
// Security Event Timeline Component
// ============================================================================

export function SecurityEventTimeline({ timeline }: SecurityEventTimelineProps) {
  // Calculate max count for scaling
  const maxCount = useMemo(() => {
    return Math.max(...timeline.map((t) => t.count), 1);
  }, [timeline]);

  // Group timeline by time period
  const groupedTimeline = useMemo(() => {
    if (timeline.length === 0) return [];

    // Determine time period (hourly, daily, etc.)
    const timeSpan =
      new Date(timeline[timeline.length - 1].timestamp).getTime() -
      new Date(timeline[0].timestamp).getTime();
    const hoursSpan = timeSpan / (1000 * 60 * 60);

    let formatLabel: (timestamp: string) => string;
    if (hoursSpan <= 24) {
      // Show hours for last 24 hours
      formatLabel = (timestamp: string) => {
        return new Date(timestamp).toLocaleTimeString([], {
          hour: '2-digit',
          minute: '2-digit',
        });
      };
    } else if (hoursSpan <= 168) {
      // Show days for last week
      formatLabel = (timestamp: string) => {
        return new Date(timestamp).toLocaleDateString([], {
          month: 'short',
          day: 'numeric',
        });
      };
    } else {
      // Show dates for longer periods
      formatLabel = (timestamp: string) => {
        return new Date(timestamp).toLocaleDateString([], {
          month: 'short',
          day: 'numeric',
        });
      };
    }

    return timeline.map((item) => ({
      ...item,
      label: formatLabel(item.timestamp),
    }));
  }, [timeline]);

  if (timeline.length === 0) {
    return (
      <div className="card">
        <div className="text-center py-12">
          <ChartBarIcon className="w-12 h-12 text-dark-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-dark-100 mb-2">No timeline data</h3>
          <p className="text-dark-400">Audit events will appear here over time</p>
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <h2 className="text-lg font-semibold text-dark-100 mb-6 flex items-center gap-3">
        <ChartBarIcon className="w-5 h-5 text-rusty-500" />
        Event Timeline
      </h2>

      {/* Timeline Chart */}
      <div className="space-y-4">
        {/* Y-axis labels */}
        <div className="flex items-end gap-2" style={{ height: '300px' }}>
          {/* Y-axis */}
          <div className="flex flex-col justify-between h-full pr-3 border-r border-dark-700">
            <span className="text-xs text-dark-400">{maxCount}</span>
            <span className="text-xs text-dark-400">{Math.floor(maxCount * 0.75)}</span>
            <span className="text-xs text-dark-400">{Math.floor(maxCount * 0.5)}</span>
            <span className="text-xs text-dark-400">{Math.floor(maxCount * 0.25)}</span>
            <span className="text-xs text-dark-400">0</span>
          </div>

          {/* Bars */}
          <div className="flex-1 flex items-end gap-1 h-full">
            {groupedTimeline.map((item, index) => {
              const height = (item.count / maxCount) * 100;
              const isHigh = item.count > maxCount * 0.7;
              const isMedium = item.count > maxCount * 0.4 && item.count <= maxCount * 0.7;

              return (
                <motion.div
                  key={index}
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: `${height}%`, opacity: 1 }}
                  transition={{ delay: index * 0.02, duration: 0.3 }}
                  className="flex-1 min-w-0 group relative"
                >
                  <div
                    className={clsx(
                      'w-full rounded-t transition-colors cursor-pointer',
                      isHigh
                        ? 'bg-danger-500 hover:bg-danger-400'
                        : isMedium
                        ? 'bg-warning-500 hover:bg-warning-400'
                        : 'bg-rusty-500 hover:bg-rusty-400'
                    )}
                    style={{ height: '100%' }}
                  >
                    {/* Tooltip */}
                    <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-10">
                      <div className="bg-dark-900 border border-dark-700 rounded-lg shadow-xl p-2 whitespace-nowrap">
                        <div className="text-xs text-dark-400">{item.label}</div>
                        <div className="text-sm font-medium text-dark-100">
                          {item.count} events
                        </div>
                      </div>
                    </div>
                  </div>
                </motion.div>
              );
            })}
          </div>
        </div>

        {/* X-axis labels */}
        <div className="flex items-center gap-2 pl-14">
          <div className="flex-1 flex justify-between">
            {groupedTimeline
              .filter((_, i) => {
                // Show every nth label to avoid crowding
                const step = Math.ceil(groupedTimeline.length / 8);
                return i % step === 0 || i === groupedTimeline.length - 1;
              })
              .map((item, index) => (
                <span
                  key={index}
                  className="text-xs text-dark-400"
                  style={{ flexBasis: 0, flexGrow: 1 }}
                >
                  {item.label}
                </span>
              ))}
          </div>
        </div>

        {/* Legend */}
        <div className="flex items-center justify-center gap-6 pt-4 border-t border-dark-700">
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded bg-rusty-500" />
            <span className="text-xs text-dark-400">Low Activity</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded bg-warning-500" />
            <span className="text-xs text-dark-400">Medium Activity</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded bg-danger-500" />
            <span className="text-xs text-dark-400">High Activity</span>
          </div>
        </div>

        {/* Summary Stats */}
        <div className="grid grid-cols-3 gap-4 pt-4 border-t border-dark-700">
          <div className="text-center">
            <div className="text-2xl font-bold text-dark-100">
              {timeline.reduce((sum, item) => sum + item.count, 0)}
            </div>
            <div className="text-xs text-dark-400 mt-1">Total Events</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-dark-100">
              {Math.round(
                timeline.reduce((sum, item) => sum + item.count, 0) / timeline.length
              )}
            </div>
            <div className="text-xs text-dark-400 mt-1">Average per Period</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-dark-100">{maxCount}</div>
            <div className="text-xs text-dark-400 mt-1">Peak Activity</div>
          </div>
        </div>
      </div>
    </div>
  );
}
