import { motion } from 'framer-motion';
import type { SystemMetrics } from '../../types';

interface PerformanceGaugesProps {
  metrics: SystemMetrics;
  className?: string;
}

interface GaugeProps {
  label: string;
  value: number;
  max?: number;
  unit?: string;
  trend?: 'up' | 'down' | 'stable';
}

function Gauge({ label, value, max = 100, unit = '%', trend }: GaugeProps) {
  const percentage = (value / max) * 100;
  const clampedPercentage = Math.min(Math.max(percentage, 0), 100);

  // Color based on usage
  const getColor = () => {
    if (clampedPercentage >= 90) return 'text-red-500';
    if (clampedPercentage >= 75) return 'text-yellow-500';
    if (clampedPercentage >= 50) return 'text-blue-500';
    return 'text-green-500';
  };

  const getStrokeColor = () => {
    if (clampedPercentage >= 90) return '#ef4444';
    if (clampedPercentage >= 75) return '#eab308';
    if (clampedPercentage >= 50) return '#3b82f6';
    return '#10b981';
  };

  const getTrendIcon = () => {
    if (!trend) return null;
    if (trend === 'up') return '↑';
    if (trend === 'down') return '↓';
    return '→';
  };

  const radius = 40;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (clampedPercentage / 100) * circumference;

  return (
    <div className="flex flex-col items-center p-4">
      <div className="relative w-32 h-32">
        <svg className="transform -rotate-90 w-32 h-32">
          {/* Background circle */}
          <circle
            cx="64"
            cy="64"
            r={radius}
            stroke="currentColor"
            strokeWidth="8"
            fill="none"
            className="text-gray-700"
          />
          {/* Progress circle */}
          <motion.circle
            cx="64"
            cy="64"
            r={radius}
            stroke={getStrokeColor()}
            strokeWidth="8"
            fill="none"
            strokeDasharray={circumference}
            strokeDashoffset={strokeDashoffset}
            strokeLinecap="round"
            initial={{ strokeDashoffset: circumference }}
            animate={{ strokeDashoffset }}
            transition={{ duration: 1, ease: 'easeInOut' }}
          />
        </svg>

        {/* Center text */}
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className={`text-2xl font-bold ${getColor()}`}>
            {value.toFixed(1)}
          </span>
          <span className="text-xs text-gray-400">{unit}</span>
        </div>
      </div>

      <div className="mt-2 text-center">
        <div className="text-sm font-medium text-gray-200">{label}</div>
        {trend && (
          <div
            className={`text-xs ${
              trend === 'up' ? 'text-red-400' : trend === 'down' ? 'text-green-400' : 'text-gray-400'
            }`}
          >
            {getTrendIcon()} {trend}
          </div>
        )}
      </div>
    </div>
  );
}

export function PerformanceGauges({ metrics, className = '' }: PerformanceGaugesProps) {
  return (
    <div className={`grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 ${className}`}>
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
        className="bg-gray-800 rounded-lg shadow-lg"
      >
        <Gauge
          label="CPU Usage"
          value={metrics.cpu.usage}
          max={100}
          unit="%"
        />
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.2 }}
        className="bg-gray-800 rounded-lg shadow-lg"
      >
        <Gauge
          label="Memory Usage"
          value={metrics.memory.usagePercent}
          max={100}
          unit="%"
        />
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.3 }}
        className="bg-gray-800 rounded-lg shadow-lg"
      >
        <Gauge
          label="Disk Usage"
          value={metrics.disk.usagePercent}
          max={100}
          unit="%"
        />
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.4 }}
        className="bg-gray-800 rounded-lg shadow-lg"
      >
        <Gauge
          label="Buffer Hit Ratio"
          value={metrics.database.cacheHitRatio}
          max={100}
          unit="%"
        />
      </motion.div>
    </div>
  );
}
