import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ClockIcon,
  CheckCircleIcon,
  InformationCircleIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import type { Timestamp } from '../../types';
import { formatDate, formatTime } from '../../utils/format';

// ============================================================================
// Types
// ============================================================================

interface PointInTimeSelectorProps {
  availablePoints?: Timestamp[];
  minDate?: Timestamp;
  maxDate?: Timestamp;
  value?: Timestamp;
  onChange: (timestamp: Timestamp) => void;
  loading?: boolean;
}

// ============================================================================
// PointInTimeSelector Component
// ============================================================================

export function PointInTimeSelector({
  availablePoints = [],
  minDate,
  maxDate,
  value,
  onChange,
  loading = false,
}: PointInTimeSelectorProps) {
  const [selectedDate, setSelectedDate] = useState<string>('');
  const [selectedTime, setSelectedTime] = useState<string>('');
  const [showTimeline, setShowTimeline] = useState(false);

  // Initialize from value
  useEffect(() => {
    if (value) {
      const date = new Date(value);
      setSelectedDate(date.toISOString().split('T')[0]);
      setSelectedTime(
        date.toTimeString().split(' ')[0].substring(0, 5) // HH:MM format
      );
    } else if (maxDate) {
      const date = new Date(maxDate);
      setSelectedDate(date.toISOString().split('T')[0]);
      setSelectedTime(
        date.toTimeString().split(' ')[0].substring(0, 5)
      );
    }
  }, [value, maxDate]);

  // Update parent when date/time changes
  useEffect(() => {
    if (selectedDate && selectedTime) {
      const timestamp = new Date(`${selectedDate}T${selectedTime}:00`).toISOString();
      onChange(timestamp);
    }
  }, [selectedDate, selectedTime, onChange]);

  // Get min/max for date input
  const minDateStr = minDate ? new Date(minDate).toISOString().split('T')[0] : undefined;
  const maxDateStr = maxDate ? new Date(maxDate).toISOString().split('T')[0] : undefined;

  // Filter recovery points for selected date
  const pointsForSelectedDate = availablePoints.filter((point) => {
    const pointDate = new Date(point).toISOString().split('T')[0];
    return pointDate === selectedDate;
  });

  // Group points by hour
  const pointsByHour = pointsForSelectedDate.reduce((acc, point) => {
    const hour = new Date(point).getHours();
    if (!acc[hour]) {
      acc[hour] = [];
    }
    acc[hour].push(point);
    return acc;
  }, {} as Record<number, Timestamp[]>);

  const handleSelectPoint = (point: Timestamp) => {
    const date = new Date(point);
    setSelectedDate(date.toISOString().split('T')[0]);
    setSelectedTime(date.toTimeString().split(' ')[0].substring(0, 5));
    onChange(point);
  };

  return (
    <div className="space-y-6">
      {/* Info Banner */}
      <div className="p-4 bg-info-500/10 border border-info-500/30 rounded-lg">
        <div className="flex items-start gap-3">
          <InformationCircleIcon className="w-5 h-5 text-info-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <h4 className="text-sm font-medium text-info-300 mb-1">
              Point-in-Time Recovery
            </h4>
            <p className="text-sm text-info-200">
              Select a specific point in time to restore your database. Available recovery
              points are shown based on your backup and WAL archive history.
            </p>
          </div>
        </div>
      </div>

      {/* Date & Time Selectors */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Date Picker */}
        <div>
          <label className="block text-sm font-medium text-dark-200 mb-2">
            Select Date
          </label>
          <input
            type="date"
            value={selectedDate}
            onChange={(e) => setSelectedDate(e.target.value)}
            min={minDateStr}
            max={maxDateStr}
            className="input w-full"
            disabled={loading}
          />
          {minDateStr && maxDateStr && (
            <p className="mt-1 text-xs text-dark-400">
              Available: {formatDate(minDateStr, false)} to {formatDate(maxDateStr, false)}
            </p>
          )}
        </div>

        {/* Time Picker */}
        <div>
          <label className="block text-sm font-medium text-dark-200 mb-2">
            Select Time (24h)
          </label>
          <input
            type="time"
            value={selectedTime}
            onChange={(e) => setSelectedTime(e.target.value)}
            className="input w-full"
            disabled={loading}
          />
          {selectedDate && selectedTime && (
            <p className="mt-1 text-xs text-dark-400">
              {formatDate(`${selectedDate}T${selectedTime}:00`)}
            </p>
          )}
        </div>
      </div>

      {/* Available Recovery Points */}
      {availablePoints.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-3">
            <label className="text-sm font-medium text-dark-200">
              Available Recovery Points
            </label>
            <button
              type="button"
              onClick={() => setShowTimeline(!showTimeline)}
              className="text-sm text-rusty-400 hover:text-rusty-300"
            >
              {showTimeline ? 'Hide' : 'Show'} Timeline
            </button>
          </div>

          <AnimatePresence>
            {showTimeline && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
                exit={{ opacity: 0, height: 0 }}
                className="overflow-hidden"
              >
                {loading ? (
                  <div className="p-8 text-center">
                    <div className="inline-block w-6 h-6 border-4 border-dark-600 border-t-rusty-500 rounded-full animate-spin" />
                    <p className="mt-3 text-sm text-dark-400">
                      Loading recovery points...
                    </p>
                  </div>
                ) : pointsForSelectedDate.length === 0 ? (
                  <div className="p-6 text-center bg-dark-750 border border-dark-600 rounded-lg">
                    <ClockIcon className="w-8 h-8 text-dark-600 mx-auto mb-2" />
                    <p className="text-sm text-dark-400">
                      No recovery points available for selected date
                    </p>
                  </div>
                ) : (
                  <div className="card p-4">
                    {/* Timeline */}
                    <div className="space-y-4">
                      {Object.entries(pointsByHour)
                        .sort(([a], [b]) => parseInt(b) - parseInt(a))
                        .map(([hour, points]) => (
                          <div key={hour}>
                            <div className="text-xs font-medium text-dark-400 mb-2">
                              {String(hour).padStart(2, '0')}:00
                            </div>
                            <div className="pl-4 border-l-2 border-dark-600 space-y-2">
                              {points.map((point) => {
                                const isSelected =
                                  value && new Date(value).getTime() === new Date(point).getTime();
                                return (
                                  <button
                                    key={point}
                                    type="button"
                                    onClick={() => handleSelectPoint(point)}
                                    className={clsx(
                                      'flex items-center gap-3 p-3 rounded-lg border-2 transition-colors w-full text-left',
                                      isSelected
                                        ? 'border-rusty-500 bg-rusty-500/10'
                                        : 'border-dark-600 hover:border-dark-500 bg-dark-750'
                                    )}
                                  >
                                    <div
                                      className={clsx(
                                        'w-2 h-2 rounded-full flex-shrink-0',
                                        isSelected ? 'bg-rusty-500' : 'bg-dark-500'
                                      )}
                                    />
                                    <div className="flex-1">
                                      <div className="flex items-center gap-2">
                                        <span className="text-sm font-medium text-dark-100">
                                          {formatTime(point)}
                                        </span>
                                        {isSelected && (
                                          <CheckCircleIcon className="w-4 h-4 text-rusty-400" />
                                        )}
                                      </div>
                                      <p className="text-xs text-dark-400 mt-0.5">
                                        {formatDate(point)}
                                      </p>
                                    </div>
                                  </button>
                                );
                              })}
                            </div>
                          </div>
                        ))}
                    </div>
                  </div>
                )}
              </motion.div>
            )}
          </AnimatePresence>

          {/* Point Count Summary */}
          <div className="mt-3 p-3 bg-dark-750 border border-dark-600 rounded-lg">
            <div className="flex items-center justify-between text-sm">
              <span className="text-dark-400">Total recovery points:</span>
              <span className="font-medium text-dark-100">
                {availablePoints.length}
              </span>
            </div>
            {selectedDate && (
              <div className="flex items-center justify-between text-sm mt-2 pt-2 border-t border-dark-600">
                <span className="text-dark-400">Points for selected date:</span>
                <span className="font-medium text-dark-100">
                  {pointsForSelectedDate.length}
                </span>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Selected Point Summary */}
      {selectedDate && selectedTime && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="p-4 bg-rusty-500/10 border border-rusty-500/30 rounded-lg"
        >
          <div className="flex items-center gap-3">
            <ClockIcon className="w-5 h-5 text-rusty-400" />
            <div>
              <p className="text-sm font-medium text-rusty-300">
                Selected Recovery Point
              </p>
              <p className="text-sm text-rusty-200 mt-0.5">
                {formatDate(`${selectedDate}T${selectedTime}:00`)}
              </p>
            </div>
          </div>
        </motion.div>
      )}
    </div>
  );
}
