// ============================================================================
// Change Data Capture & Streaming Service
// ============================================================================

import { get, post, del, patch, buildQueryParams, WS_URL } from './api';
import type {
  UUID,
  Timestamp,
  PaginatedResponse,
  PaginationParams,
} from '../types';

// ============================================================================
// Request/Response Types
// ============================================================================

export interface CDCStatus {
  enabled: boolean;
  activeSubscriptions: number;
  totalChangesCaptured: number;
  lastCapturedChange?: Timestamp;
  status: 'running' | 'paused' | 'stopped' | 'error';
  walPosition?: string;
  replicationSlot?: string;
  lag?: number;
  errorMessage?: string;
}

export interface CDCSubscription {
  id: UUID;
  name: string;
  tables: string[];
  operations: CDCOperation[];
  status: SubscriptionStatus;
  outputFormat: OutputFormat;
  filterExpression?: string;
  batchSize?: number;
  maxLatency?: number;
  deliveryGuarantee: DeliveryGuarantee;
  createdAt: Timestamp;
  updatedAt: Timestamp;
  lastDeliveredChange?: Timestamp;
  changesDelivered: number;
  changesPending: number;
}

export type CDCOperation = 'insert' | 'update' | 'delete' | 'truncate';

export type SubscriptionStatus =
  | 'active'
  | 'paused'
  | 'stopped'
  | 'error'
  | 'initializing';

export type OutputFormat = 'json' | 'avro' | 'protobuf' | 'csv';

export type DeliveryGuarantee = 'at_least_once' | 'at_most_once' | 'exactly_once';

export interface CreateSubscriptionRequest {
  name: string;
  tables: string[];
  operations?: CDCOperation[];
  outputFormat?: OutputFormat;
  filterExpression?: string;
  batchSize?: number;
  maxLatency?: number;
  deliveryGuarantee?: DeliveryGuarantee;
  destination?: SubscriptionDestination;
}

export interface SubscriptionDestination {
  type: 'webhook' | 'kafka' | 'rabbitmq' | 's3' | 'pubsub';
  config: Record<string, unknown>;
}

export interface UpdateSubscriptionRequest {
  name?: string;
  operations?: CDCOperation[];
  filterExpression?: string;
  batchSize?: number;
  maxLatency?: number;
}

export interface ChangeEvent {
  id: UUID;
  subscriptionId: UUID;
  operation: CDCOperation;
  table: string;
  schema?: string;
  timestamp: Timestamp;
  transactionId?: string;
  lsn?: string;
  before?: Record<string, unknown>;
  after?: Record<string, unknown>;
  primaryKey: Record<string, unknown>;
}

export interface ChangeEventsResponse {
  events: ChangeEvent[];
  hasMore: boolean;
  nextToken?: string;
  count: number;
}

export interface SubscriptionMetrics {
  subscriptionId: UUID;
  changesDelivered: number;
  changesPending: number;
  deliveryRate: number;
  errorRate: number;
  averageLatency: number;
  maxLatency: number;
  lastDelivery?: Timestamp;
  timeSeriesData: MetricDataPoint[];
}

export interface MetricDataPoint {
  timestamp: Timestamp;
  changesDelivered: number;
  deliveryRate: number;
  latency: number;
  errors: number;
}

export interface SubscriptionFilters extends PaginationParams {
  status?: SubscriptionStatus;
  table?: string;
  search?: string;
}

export interface StreamProcessor {
  id: UUID;
  name: string;
  type: 'filter' | 'transform' | 'aggregate' | 'join' | 'window';
  config: Record<string, unknown>;
  inputStreams: string[];
  outputStream: string;
  status: 'running' | 'paused' | 'stopped' | 'error';
  eventsProcessed: number;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface CreateStreamProcessorRequest {
  name: string;
  type: 'filter' | 'transform' | 'aggregate' | 'join' | 'window';
  config: Record<string, unknown>;
  inputStreams: string[];
  outputStream: string;
}

export interface StreamQuery {
  id: UUID;
  name: string;
  query: string;
  inputStreams: string[];
  outputStream: string;
  status: 'running' | 'paused' | 'stopped' | 'error';
  createdAt: Timestamp;
}

export interface CreateStreamQueryRequest {
  name: string;
  query: string;
  outputStream: string;
}

export interface ReplicationSlot {
  id: UUID;
  name: string;
  plugin: string;
  slotType: 'logical' | 'physical';
  database: string;
  active: boolean;
  restartLsn: string;
  confirmedFlushLsn: string;
  lagBytes: number;
  lagSeconds: number;
  createdAt: Timestamp;
}

export interface CreateReplicationSlotRequest {
  name: string;
  plugin?: string;
  slotType?: 'logical' | 'physical';
  temporary?: boolean;
}

export interface StreamWebSocket {
  connect: (
    subscriptionId: UUID,
    onMessage: (event: ChangeEvent) => void,
    onError?: (error: Error) => void
  ) => void;
  disconnect: () => void;
  pause: () => void;
  resume: () => void;
}

// ============================================================================
// CDC Status APIs
// ============================================================================

/**
 * Get CDC system status
 */
export async function getCDCStatus(): Promise<CDCStatus> {
  const response = await get<CDCStatus>('/streams/cdc/status');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch CDC status');
  }

  return response.data;
}

/**
 * Enable CDC
 */
export async function enableCDC(): Promise<CDCStatus> {
  const response = await post<CDCStatus>('/streams/cdc/enable');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to enable CDC');
  }

  return response.data;
}

/**
 * Disable CDC
 */
export async function disableCDC(): Promise<CDCStatus> {
  const response = await post<CDCStatus>('/streams/cdc/disable');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to disable CDC');
  }

  return response.data;
}

// ============================================================================
// Subscription Management APIs
// ============================================================================

/**
 * List CDC subscriptions with optional filtering and pagination
 */
export async function listSubscriptions(
  filters?: SubscriptionFilters
): Promise<PaginatedResponse<CDCSubscription>> {
  const queryString = filters ? buildQueryParams(filters) : '';
  const response = await get<PaginatedResponse<CDCSubscription>>(
    `/streams/cdc/subscriptions${queryString}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch subscriptions');
  }

  return response.data;
}

/**
 * Create a new CDC subscription
 */
export async function createSubscription(
  request: CreateSubscriptionRequest
): Promise<CDCSubscription> {
  const response = await post<CDCSubscription>(
    '/streams/cdc/subscriptions',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create subscription');
  }

  return response.data;
}

/**
 * Get subscription details by ID
 */
export async function getSubscription(
  subscriptionId: UUID
): Promise<CDCSubscription> {
  const response = await get<CDCSubscription>(
    `/streams/cdc/subscriptions/${subscriptionId}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch subscription');
  }

  return response.data;
}

/**
 * Update a CDC subscription
 */
export async function updateSubscription(
  subscriptionId: UUID,
  request: UpdateSubscriptionRequest
): Promise<CDCSubscription> {
  const response = await patch<CDCSubscription>(
    `/streams/cdc/subscriptions/${subscriptionId}`,
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to update subscription');
  }

  return response.data;
}

/**
 * Delete a CDC subscription
 */
export async function deleteSubscription(subscriptionId: UUID): Promise<void> {
  const response = await del<void>(
    `/streams/cdc/subscriptions/${subscriptionId}`
  );

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete subscription');
  }
}

/**
 * Pause a CDC subscription
 */
export async function pauseSubscription(
  subscriptionId: UUID
): Promise<CDCSubscription> {
  const response = await post<CDCSubscription>(
    `/streams/cdc/subscriptions/${subscriptionId}/pause`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to pause subscription');
  }

  return response.data;
}

/**
 * Resume a paused CDC subscription
 */
export async function resumeSubscription(
  subscriptionId: UUID
): Promise<CDCSubscription> {
  const response = await post<CDCSubscription>(
    `/streams/cdc/subscriptions/${subscriptionId}/resume`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to resume subscription');
  }

  return response.data;
}

// ============================================================================
// Change Events APIs
// ============================================================================

/**
 * Poll for change events (for non-WebSocket clients)
 */
export async function getChanges(
  subscriptionId: UUID,
  limit: number = 100,
  nextToken?: string
): Promise<ChangeEventsResponse> {
  const params: Record<string, unknown> = { limit };
  if (nextToken) {
    params.nextToken = nextToken;
  }

  const response = await get<ChangeEventsResponse>(
    `/streams/cdc/subscriptions/${subscriptionId}/changes${buildQueryParams(params)}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch changes');
  }

  return response.data;
}

/**
 * Acknowledge change events (for exactly-once delivery)
 */
export async function acknowledgeChanges(
  subscriptionId: UUID,
  eventIds: UUID[]
): Promise<void> {
  const response = await post<void>(
    `/streams/cdc/subscriptions/${subscriptionId}/ack`,
    { eventIds }
  );

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to acknowledge changes');
  }
}

// ============================================================================
// Metrics APIs
// ============================================================================

/**
 * Get subscription metrics
 */
export async function getSubscriptionMetrics(
  subscriptionId: UUID,
  timeRange?: { start: Timestamp; end: Timestamp }
): Promise<SubscriptionMetrics> {
  const params = timeRange
    ? buildQueryParams({ start: timeRange.start, end: timeRange.end })
    : '';

  const response = await get<SubscriptionMetrics>(
    `/streams/cdc/subscriptions/${subscriptionId}/metrics${params}`
  );

  if (!response.success || !response.data) {
    throw new Error(
      response.error?.message || 'Failed to fetch subscription metrics'
    );
  }

  return response.data;
}

// ============================================================================
// Stream Processing APIs
// ============================================================================

/**
 * List stream processors
 */
export async function listStreamProcessors(): Promise<StreamProcessor[]> {
  const response = await get<StreamProcessor[]>('/streams/processors');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch stream processors');
  }

  return response.data;
}

/**
 * Create a stream processor
 */
export async function createStreamProcessor(
  request: CreateStreamProcessorRequest
): Promise<StreamProcessor> {
  const response = await post<StreamProcessor>('/streams/processors', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create stream processor');
  }

  return response.data;
}

/**
 * Delete a stream processor
 */
export async function deleteStreamProcessor(processorId: UUID): Promise<void> {
  const response = await del<void>(`/streams/processors/${processorId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete stream processor');
  }
}

/**
 * List stream queries
 */
export async function listStreamQueries(): Promise<StreamQuery[]> {
  const response = await get<StreamQuery[]>('/streams/queries');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch stream queries');
  }

  return response.data;
}

/**
 * Create a stream query
 */
export async function createStreamQuery(
  request: CreateStreamQueryRequest
): Promise<StreamQuery> {
  const response = await post<StreamQuery>('/streams/queries', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create stream query');
  }

  return response.data;
}

/**
 * Delete a stream query
 */
export async function deleteStreamQuery(queryId: UUID): Promise<void> {
  const response = await del<void>(`/streams/queries/${queryId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete stream query');
  }
}

// ============================================================================
// Replication Slot APIs
// ============================================================================

/**
 * List replication slots
 */
export async function listReplicationSlots(): Promise<ReplicationSlot[]> {
  const response = await get<ReplicationSlot[]>('/streams/replication-slots');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch replication slots');
  }

  return response.data;
}

/**
 * Create a replication slot
 */
export async function createReplicationSlot(
  request: CreateReplicationSlotRequest
): Promise<ReplicationSlot> {
  const response = await post<ReplicationSlot>(
    '/streams/replication-slots',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create replication slot');
  }

  return response.data;
}

/**
 * Delete a replication slot
 */
export async function deleteReplicationSlot(slotId: UUID): Promise<void> {
  const response = await del<void>(`/streams/replication-slots/${slotId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete replication slot');
  }
}

// ============================================================================
// WebSocket Connection for Real-time Change Streaming
// ============================================================================

/**
 * Create WebSocket connection for real-time change events
 */
export function createStreamWebSocket(): StreamWebSocket {
  let ws: WebSocket | null = null;
  let currentSubscriptionId: UUID | null = null;
  let isPaused = false;

  return {
    connect: (
      subscriptionId: UUID,
      onMessage: (event: ChangeEvent) => void,
      onError?: (error: Error) => void
    ) => {
      // Get auth token for WebSocket
      const storedAuth = localStorage.getItem('rustydb_auth');
      let token = '';
      if (storedAuth) {
        try {
          const { session } = JSON.parse(storedAuth);
          token = session?.token || '';
        } catch {
          // Invalid auth, continue without token
        }
      }

      currentSubscriptionId = subscriptionId;
      ws = new WebSocket(`${WS_URL}/streams/cdc/${subscriptionId}?token=${token}`);

      ws.onopen = () => {
        console.log(`[CDC Stream WS] Connected to subscription ${subscriptionId}`);
        if (isPaused) {
          ws?.send(JSON.stringify({ type: 'pause' }));
        }
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);

          // Handle different message types
          if (data.type === 'change_event') {
            onMessage(data.event as ChangeEvent);
          } else if (data.type === 'error') {
            if (onError) {
              onError(new Error(data.message || 'Stream error'));
            }
          } else if (data.type === 'heartbeat') {
            // Heartbeat to keep connection alive
            ws?.send(JSON.stringify({ type: 'heartbeat_ack' }));
          }
        } catch (error) {
          console.error('[CDC Stream WS] Failed to parse message:', error);
          if (onError) {
            onError(
              error instanceof Error ? error : new Error('Failed to parse message')
            );
          }
        }
      };

      ws.onerror = (error) => {
        console.error('[CDC Stream WS] Error:', error);
        if (onError) {
          onError(new Error('WebSocket connection error'));
        }
      };

      ws.onclose = (event) => {
        console.log('[CDC Stream WS] Disconnected', event.code, event.reason);

        // Auto-reconnect after 5 seconds if not a normal closure
        if (event.code !== 1000 && currentSubscriptionId) {
          setTimeout(() => {
            if (ws?.readyState === WebSocket.CLOSED && currentSubscriptionId) {
              this.connect(currentSubscriptionId, onMessage, onError);
            }
          }, 5000);
        }
      };
    },

    disconnect: () => {
      if (ws) {
        ws.close(1000, 'Client disconnect');
        ws = null;
        currentSubscriptionId = null;
        isPaused = false;
      }
    },

    pause: () => {
      isPaused = true;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'pause' }));
      }
    },

    resume: () => {
      isPaused = false;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'resume' }));
      }
    },
  };
}

// ============================================================================
// Export Service Object (Alternative Pattern)
// ============================================================================

export const streamService = {
  // CDC Status
  getCDCStatus,
  enableCDC,
  disableCDC,

  // Subscriptions
  listSubscriptions,
  createSubscription,
  getSubscription,
  updateSubscription,
  deleteSubscription,
  pauseSubscription,
  resumeSubscription,

  // Changes
  getChanges,
  acknowledgeChanges,

  // Metrics
  getSubscriptionMetrics,

  // Stream Processing
  listStreamProcessors,
  createStreamProcessor,
  deleteStreamProcessor,
  listStreamQueries,
  createStreamQuery,
  deleteStreamQuery,

  // Replication Slots
  listReplicationSlots,
  createReplicationSlot,
  deleteReplicationSlot,

  // WebSocket
  createStreamWebSocket,
};
