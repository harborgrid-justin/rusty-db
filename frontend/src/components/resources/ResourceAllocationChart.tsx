import { useMemo } from 'react';
import { ResourceGroup, ResourceUsage } from '../../types';
import { Card, CardHeader } from '../common/Card';

// ============================================================================
// Resource Allocation Chart Component
// Visualizes resource allocation across groups
// ============================================================================

export interface ResourceAllocationChartProps {
  groups: ResourceGroup[];
  usageData?: Map<string, ResourceUsage>;
  resourceType: 'cpu' | 'memory' | 'connections';
  showLegend?: boolean;
}

export function ResourceAllocationChart({
  groups,
  usageData,
  resourceType,
  showLegend = true,
}: ResourceAllocationChartProps) {
  const chartData = useMemo(() => {
    const colors = [
      '#ed7519', // rusty-500
      '#10b981', // success-500
      '#f59e0b', // warning-500
      '#3b82f6', // info-500
      '#8b5cf6', // purple-500
      '#ec4899', // pink-500
      '#14b8a6', // teal-500
    ];

    let total = 0;
    let allocated = 0;

    groups.forEach((group) => {
      switch (resourceType) {
        case 'cpu':
          total += 100; // Assume 100% per group for visualization
          allocated += group.cpuLimit;
          break;
        case 'memory':
          total += group.memoryLimit * 2; // Assume double for total capacity
          allocated += group.memoryLimit;
          break;
        case 'connections':
          total += group.maxConnections * 1.5;
          allocated += group.maxConnections;
          break;
      }
    });

    const segments = groups.map((group, index) => {
      let value = 0;
      let usage = 0;

      switch (resourceType) {
        case 'cpu':
          value = group.cpuLimit;
          usage = usageData?.get(group.id)?.cpuUsage || 0;
          break;
        case 'memory':
          value = group.memoryLimit;
          usage = usageData?.get(group.id)?.memoryUsage || 0;
          break;
        case 'connections':
          value = group.maxConnections;
          usage = usageData?.get(group.id)?.activeConnections || 0;
          break;
      }

      const percentage = (value / total) * 100;
      const usagePercentage = value > 0 ? (usage / value) * 100 : 0;

      return {
        id: group.id,
        name: group.name,
        value,
        usage,
        percentage,
        usagePercentage,
        color: colors[index % colors.length],
      };
    });

    const available = total - allocated;
    const availablePercentage = (available / total) * 100;

    return {
      segments,
      available,
      availablePercentage,
      total,
      allocated,
    };
  }, [groups, usageData, resourceType]);

  const formatValue = (value: number) => {
    switch (resourceType) {
      case 'cpu':
        return `${value.toFixed(1)}%`;
      case 'memory':
        return `${(value / (1024 * 1024 * 1024)).toFixed(2)} GB`;
      case 'connections':
        return Math.round(value).toString();
      default:
        return value.toString();
    }
  };

  const resourceLabel = {
    cpu: 'CPU',
    memory: 'Memory',
    connections: 'Connections',
  }[resourceType];

  // Calculate donut chart paths
  const size = 200;
  const strokeWidth = 40;
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const center = size / 2;

  let currentOffset = 0;

  return (
    <Card>
      <CardHeader title={`${resourceLabel} Allocation`} />

      <div className="flex flex-col md:flex-row items-center gap-8">
        {/* Donut Chart */}
        <div className="relative">
          <svg width={size} height={size} className="transform -rotate-90">
            {/* Background circle */}
            <circle
              cx={center}
              cy={center}
              r={radius}
              fill="none"
              stroke="currentColor"
              strokeWidth={strokeWidth}
              className="text-dark-700"
            />

            {/* Segments */}
            {chartData.segments.map((segment) => {
              const segmentLength = (segment.percentage / 100) * circumference;
              const path = (
                <circle
                  key={segment.id}
                  cx={center}
                  cy={center}
                  r={radius}
                  fill="none"
                  stroke={segment.color}
                  strokeWidth={strokeWidth}
                  strokeDasharray={`${segmentLength} ${circumference - segmentLength}`}
                  strokeDashoffset={-currentOffset}
                  className="transition-all duration-300"
                />
              );
              currentOffset += segmentLength;
              return path;
            })}
          </svg>

          {/* Center text */}
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <div className="text-2xl font-bold text-dark-100">
              {((chartData.allocated / chartData.total) * 100).toFixed(0)}%
            </div>
            <div className="text-xs text-dark-400">Allocated</div>
          </div>
        </div>

        {/* Legend */}
        {showLegend && (
          <div className="flex-1 space-y-3">
            {chartData.segments.map((segment) => (
              <div key={segment.id} className="flex items-center justify-between">
                <div className="flex items-center gap-3 flex-1">
                  <div
                    className="w-3 h-3 rounded-full flex-shrink-0"
                    style={{ backgroundColor: segment.color }}
                  />
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium text-dark-200 truncate">
                      {segment.name}
                    </div>
                    <div className="text-xs text-dark-400">
                      {formatValue(segment.usage)} / {formatValue(segment.value)}
                    </div>
                  </div>
                </div>
                <div className="text-sm font-semibold text-dark-300 ml-4">
                  {segment.percentage.toFixed(1)}%
                </div>
              </div>
            ))}

            {chartData.availablePercentage > 0 && (
              <div className="flex items-center justify-between pt-3 border-t border-dark-700">
                <div className="flex items-center gap-3">
                  <div className="w-3 h-3 rounded-full bg-dark-700 flex-shrink-0" />
                  <div>
                    <div className="text-sm font-medium text-dark-200">Available</div>
                    <div className="text-xs text-dark-400">
                      {formatValue(chartData.available)}
                    </div>
                  </div>
                </div>
                <div className="text-sm font-semibold text-dark-300">
                  {chartData.availablePercentage.toFixed(1)}%
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </Card>
  );
}
