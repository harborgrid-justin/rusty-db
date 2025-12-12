// ============================================================================
// RustyDB Activity Feed Component
// Recent activity timeline with filtering, pagination, and real-time updates
// ============================================================================

import React, { useState, useMemo } from 'react';
import {
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  InformationCircleIcon,
  ChevronDownIcon,
  ChevronUpIcon,
  FunnelIcon,
} from '@heroicons/react/24/solid';
import {
  CommandLineIcon,
  CircleStackIcon,
  UserIcon,
  Cog6ToothIcon,
  BellAlertIcon,
  ClockIcon,
  ArrowPathIcon,
  TableCellsIcon,
  ServerIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';
import { format, parseISO, formatDistance } from 'date-fns';
import { ActivityEvent, ActivityEventType } from '@/services/metricsService.ts';

// ============================================================================
// Types
// ============================================================================

export interface ActivityFeedProps {
  events: ActivityEvent[];
  isLoading?: boolean;
  error?: string | null;
  onLoadMore?: () => void;
  hasMore?: boolean;
  className?: string;
  maxHeight?: string;
  showFilter?: boolean;
  showTimestamps?: boolean;
  realtime?: boolean;
}

// ============================================================================
// Event Type Configuration
// ============================================================================

const EVENT_TYPE_CONFIG: Record<
  ActivityEventType,
  {
    icon: React.ComponentType<{ className?: string }>;
    label: string;
    color: string;
  }
> = {
  query_executed: {
    icon: CommandLineIcon,
    label: 'Query Executed',
    color: 'text-blue-600 bg-blue-50',
  },
  backup_created: {
    icon: CircleStackIcon,
    label: 'Backup Created',
    color: 'text-green-600 bg-green-50',
  },
  backup_failed: {
    icon: XCircleIcon,
    label: 'Backup Failed',
    color: 'text-red-600 bg-red-50',
  },
  user_login: {
    icon: UserIcon,
    label: 'User Login',
    color: 'text-purple-600 bg-purple-50',
  },
  user_logout: {
    icon: UserIcon,
    label: 'User Logout',
    color: 'text-gray-600 bg-gray-50',
  },
  configuration_changed: {
    icon: Cog6ToothIcon,
    label: 'Configuration Changed',
    color: 'text-orange-600 bg-orange-50',
  },
  alert_triggered: {
    icon: BellAlertIcon,
    label: 'Alert Triggered',
    color: 'text-red-600 bg-red-50',
  },
  alert_resolved: {
    icon: CheckCircleIcon,
    label: 'Alert Resolved',
    color: 'text-green-600 bg-green-50',
  },
  connection_limit_reached: {
    icon: ExclamationTriangleIcon,
    label: 'Connection Limit Reached',
    color: 'text-yellow-600 bg-yellow-50',
  },
  slow_query_detected: {
    icon: ClockIcon,
    label: 'Slow Query Detected',
    color: 'text-orange-600 bg-orange-50',
  },
  replication_lag: {
    icon: ArrowPathIcon,
    label: 'Replication Lag',
    color: 'text-yellow-600 bg-yellow-50',
  },
  failover_completed: {
    icon: ServerIcon,
    label: 'Failover Completed',
    color: 'text-purple-600 bg-purple-50',
  },
  index_created: {
    icon: TableCellsIcon,
    label: 'Index Created',
    color: 'text-blue-600 bg-blue-50',
  },
  table_created: {
    icon: TableCellsIcon,
    label: 'Table Created',
    color: 'text-blue-600 bg-blue-50',
  },
  maintenance_started: {
    icon: Cog6ToothIcon,
    label: 'Maintenance Started',
    color: 'text-indigo-600 bg-indigo-50',
  },
  maintenance_completed: {
    icon: CheckCircleIcon,
    label: 'Maintenance Completed',
    color: 'text-green-600 bg-green-50',
  },
};

// ============================================================================
// Main Component
// ============================================================================

export const ActivityFeed: React.FC<ActivityFeedProps> = ({
  events,
  isLoading = false,
  error = null,
  onLoadMore,
  hasMore = false,
  className,
  maxHeight = '600px',
  showFilter = true,
  showTimestamps = true,
  realtime = false,
}) => {
  const [selectedTypes, setSelectedTypes] = useState<Set<ActivityEventType>>(new Set());
  const [showFilterMenu, setShowFilterMenu] = useState(false);
  const [expandedEvents, setExpandedEvents] = useState<Set<string>>(new Set());

  // Filter events by selected types
  const filteredEvents = useMemo(() => {
    if (selectedTypes.size === 0) return events;
    return events.filter((event) => selectedTypes.has(event.type));
  }, [events, selectedTypes]);

  // Toggle event expansion
  const toggleEventExpansion = (eventId: string) => {
    setExpandedEvents((prev) => {
      const next = new Set(prev);
      if (next.has(eventId)) {
        next.delete(eventId);
      } else {
        next.add(eventId);
      }
      return next;
    });
  };

  // Toggle filter
  const toggleFilter = (type: ActivityEventType) => {
    setSelectedTypes((prev) => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
      }
      return next;
    });
  };

  // Clear all filters
  const clearFilters = () => {
    setSelectedTypes(new Set());
  };

  // Get unique event types from events
  const availableTypes = useMemo(() => {
    const types = new Set<ActivityEventType>();
    events.forEach((event) => types.add(event.type));
    return Array.from(types).sort();
  }, [events]);

  // Loading state
  if (isLoading && events.length === 0) {
    return (
      <div
        className={clsx('bg-white rounded-lg border border-gray-200 p-6', className)}
        role="status"
        aria-label="Loading activity feed"
      >
        <div className="animate-pulse space-y-4">
          {[1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="flex items-start space-x-3">
              <div className="h-10 w-10 bg-gray-300 rounded-full" />
              <div className="flex-1 space-y-2">
                <div className="h-4 bg-gray-300 rounded w-3/4" />
                <div className="h-3 bg-gray-200 rounded w-1/2" />
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div
        className={clsx('bg-white rounded-lg border border-red-200 p-6', className)}
        role="alert"
      >
        <div className="flex items-center space-x-3">
          <XCircleIcon className="h-6 w-6 text-red-600 shrink-0" />
          <div>
            <p className="text-sm font-medium text-red-600">
              Failed to load activity feed
            </p>
            <p className="text-xs text-red-500 mt-1">{error}</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={clsx('bg-white rounded-lg border border-gray-200', className)}>
      {/* Header */}
      <div className="p-4 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <h3 className="text-lg font-semibold text-gray-900">Activity Feed</h3>
            {realtime && (
              <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full bg-green-100 text-green-700 text-xs font-medium">
                <span className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
                <span>Live</span>
              </span>
            )}
          </div>

          {showFilter && availableTypes.length > 0 && (
            <div className="relative">
              <button
                type="button"
                onClick={() => setShowFilterMenu(!showFilterMenu)}
                className={clsx(
                  'flex items-center space-x-2 px-3 py-2 text-sm font-medium rounded-lg',
                  'border border-gray-300 hover:bg-gray-50 transition-colors',
                  'focus:outline-none focus:ring-2 focus:ring-blue-500',
                  selectedTypes.size > 0 && 'bg-blue-50 border-blue-300 text-blue-700'
                )}
              >
                <FunnelIcon className="h-4 w-4" />
                <span>Filter</span>
                {selectedTypes.size > 0 && (
                  <span className="px-1.5 py-0.5 rounded-full bg-blue-600 text-white text-xs">
                    {selectedTypes.size}
                  </span>
                )}
              </button>

              {showFilterMenu && (
                <FilterMenu
                  availableTypes={availableTypes}
                  selectedTypes={selectedTypes}
                  onToggle={toggleFilter}
                  onClear={clearFilters}
                  onClose={() => setShowFilterMenu(false)}
                />
              )}
            </div>
          )}
        </div>
      </div>

      {/* Events List */}
      <div
        className="overflow-y-auto"
        style={{ maxHeight }}
        role="feed"
        aria-label="Activity events"
      >
        {filteredEvents.length === 0 ? (
          <div className="p-8 text-center">
            <InformationCircleIcon className="mx-auto h-12 w-12 text-gray-400" />
            <p className="mt-2 text-sm font-medium text-gray-900">No activity</p>
            <p className="mt-1 text-sm text-gray-500">
              {selectedTypes.size > 0
                ? 'No events match the selected filters'
                : 'No recent activity to display'}
            </p>
            {selectedTypes.size > 0 && (
              <button
                type="button"
                onClick={clearFilters}
                className="mt-4 text-sm text-blue-600 hover:text-blue-700 font-medium"
              >
                Clear filters
              </button>
            )}
          </div>
        ) : (
          <div className="divide-y divide-gray-200">
            {filteredEvents.map((event, index) => (
              <ActivityEventItem
                key={event.id}
                event={event}
                isExpanded={expandedEvents.has(event.id)}
                onToggleExpand={() => toggleEventExpansion(event.id)}
                showTimestamp={showTimestamps}
                isNew={index === 0 && realtime}
              />
            ))}
          </div>
        )}

        {/* Load More */}
        {hasMore && onLoadMore && (
          <div className="p-4 border-t border-gray-200">
            <button
              type="button"
              onClick={onLoadMore}
              disabled={isLoading}
              className={clsx(
                'w-full px-4 py-2 text-sm font-medium text-blue-600 bg-blue-50 rounded-lg',
                'hover:bg-blue-100 transition-colors',
                'focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2',
                'disabled:opacity-50 disabled:cursor-not-allowed'
              )}
            >
              {isLoading ? 'Loading...' : 'Load more'}
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

// ============================================================================
// Activity Event Item Component
// ============================================================================

interface ActivityEventItemProps {
  event: ActivityEvent;
  isExpanded: boolean;
  onToggleExpand: () => void;
  showTimestamp: boolean;
  isNew?: boolean;
}

const ActivityEventItem: React.FC<ActivityEventItemProps> = ({
  event,
  isExpanded,
  onToggleExpand,
  showTimestamp,
  isNew = false,
}) => {
  const config = EVENT_TYPE_CONFIG[event.type] || {
    icon: InformationCircleIcon,
    label: event.type,
    color: 'text-gray-600 bg-gray-50',
  };

  const Icon = config.icon;

  const getSeverityIcon = () => {
    switch (event.severity) {
      case 'success':
        return <CheckCircleIcon className="h-4 w-4 text-green-600" />;
      case 'warning':
        return <ExclamationTriangleIcon className="h-4 w-4 text-yellow-600" />;
      case 'error':
        return <XCircleIcon className="h-4 w-4 text-red-600" />;
      default:
        return <InformationCircleIcon className="h-4 w-4 text-blue-600" />;
    }
  };

  const hasExpandableContent =
    event.metadata && Object.keys(event.metadata).length > 0;

  return (
    <div
      className={clsx(
        'p-4 hover:bg-gray-50 transition-colors',
        isNew && 'bg-blue-50 animate-pulse'
      )}
    >
      <div className="flex items-start space-x-3">
        {/* Icon */}
        <div className={clsx('rounded-full p-2 shrink-0', config.color)}>
          <Icon className="h-5 w-5" />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between">
            <div className="flex-1 min-w-0">
              <div className="flex items-center space-x-2">
                {getSeverityIcon()}
                <h4 className="text-sm font-medium text-gray-900 truncate">
                  {event.title}
                </h4>
              </div>

              <p className="mt-1 text-sm text-gray-600 line-clamp-2">
                {event.description}
              </p>

              <div className="mt-2 flex items-center space-x-4 text-xs text-gray-500">
                {showTimestamp && (
                  <span>
                    {formatDistance(parseISO(event.timestamp), new Date(), {
                      addSuffix: true,
                    })}
                  </span>
                )}
                {event.username && <span>by {event.username}</span>}
                <span className="px-2 py-0.5 rounded-full bg-gray-100 text-gray-700">
                  {config.label}
                </span>
              </div>
            </div>

            {/* Expand button */}
            {hasExpandableContent && (
              <button
                type="button"
                onClick={onToggleExpand}
                className="ml-2 p-1 text-gray-400 hover:text-gray-600 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
                aria-label={isExpanded ? 'Collapse details' : 'Expand details'}
              >
                {isExpanded ? (
                  <ChevronUpIcon className="h-5 w-5" />
                ) : (
                  <ChevronDownIcon className="h-5 w-5" />
                )}
              </button>
            )}
          </div>

          {/* Expanded details */}
          {isExpanded && hasExpandableContent && (
            <div className="mt-3 p-3 bg-gray-50 rounded-lg">
              <h5 className="text-xs font-semibold text-gray-700 mb-2">Details</h5>
              <dl className="space-y-1">
                {Object.entries(event.metadata!).map(([key, value]) => (
                  <div key={key} className="flex text-xs">
                    <dt className="font-medium text-gray-600 min-w-[100px]">
                      {key}:
                    </dt>
                    <dd className="text-gray-900 ml-2 break-all">
                      {typeof value === 'object'
                        ? JSON.stringify(value)
                        : String(value)}
                    </dd>
                  </div>
                ))}
              </dl>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

// ============================================================================
// Filter Menu Component
// ============================================================================

interface FilterMenuProps {
  availableTypes: ActivityEventType[];
  selectedTypes: Set<ActivityEventType>;
  onToggle: (type: ActivityEventType) => void;
  onClear: () => void;
  onClose: () => void;
}

const FilterMenu: React.FC<FilterMenuProps> = ({
  availableTypes,
  selectedTypes,
  onToggle,
  onClear,
  onClose,
}) => {
  return (
    <>
      {/* Backdrop */}
      <div className="fixed inset-0 z-10" onClick={onClose} />

      {/* Menu */}
      <div className="absolute right-0 mt-2 w-64 bg-white rounded-lg shadow-lg border border-gray-200 z-20">
        <div className="p-2">
          <div className="flex items-center justify-between p-2">
            <span className="text-sm font-semibold text-gray-900">
              Filter by type
            </span>
            {selectedTypes.size > 0 && (
              <button
                type="button"
                onClick={onClear}
                className="text-xs text-blue-600 hover:text-blue-700 font-medium"
              >
                Clear all
              </button>
            )}
          </div>

          <div className="mt-2 space-y-1 max-h-64 overflow-y-auto">
            {availableTypes.map((type) => {
              const config = EVENT_TYPE_CONFIG[type];
              const isSelected = selectedTypes.has(type);

              return (
                <label
                  key={type}
                  className="flex items-center p-2 rounded hover:bg-gray-50 cursor-pointer"
                >
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onChange={() => onToggle(type)}
                    className="h-4 w-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                  />
                  <span className="ml-3 text-sm text-gray-700">{config.label}</span>
                </label>
              );
            })}
          </div>
        </div>
      </div>
    </>
  );
};

// ============================================================================
// Export
// ============================================================================

export default ActivityFeed;
