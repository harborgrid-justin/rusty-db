// ============================================================================
// Graph Database Service
// ============================================================================

import { get, post, del, patch, buildQueryParams } from './api';
import type {
  UUID,
  Timestamp,
  PaginatedResponse,
  PaginationParams,
} from '../types';

// ============================================================================
// Request/Response Types
// ============================================================================

export interface GraphNode {
  id: UUID;
  labels: string[];
  properties: Record<string, unknown>;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface GraphEdge {
  id: UUID;
  type: string;
  sourceId: UUID;
  targetId: UUID;
  properties: Record<string, unknown>;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface CreateNodeRequest {
  labels: string[];
  properties?: Record<string, unknown>;
}

export interface UpdateNodeRequest {
  labels?: string[];
  properties?: Record<string, unknown>;
}

export interface CreateEdgeRequest {
  type: string;
  sourceId: UUID;
  targetId: UUID;
  properties?: Record<string, unknown>;
}

export interface UpdateEdgeRequest {
  type?: string;
  properties?: Record<string, unknown>;
}

export interface GraphQueryRequest {
  query: string;
  parameters?: Record<string, unknown>;
  timeout?: number;
}

export interface GraphQueryResponse {
  results: GraphQueryResult[];
  executionTime: number;
  rowsAffected?: number;
  plan?: QueryExecutionPlan;
}

export interface GraphQueryResult {
  nodes?: GraphNode[];
  edges?: GraphEdge[];
  paths?: GraphPath[];
  data?: Record<string, unknown>;
}

export interface GraphPath {
  nodes: GraphNode[];
  edges: GraphEdge[];
  length: number;
}

export interface QueryExecutionPlan {
  steps: PlanStep[];
  estimatedCost: number;
  actualCost?: number;
}

export interface PlanStep {
  operation: string;
  description: string;
  estimatedRows: number;
  actualRows?: number;
  cost: number;
}

export interface NodeFilters extends PaginationParams {
  labels?: string[];
  properties?: Record<string, unknown>;
  search?: string;
}

export interface EdgeFilters extends PaginationParams {
  types?: string[];
  sourceId?: UUID;
  targetId?: UUID;
  properties?: Record<string, unknown>;
}

export interface ShortestPathRequest {
  sourceId: UUID;
  targetId: UUID;
  maxDepth?: number;
  relationshipTypes?: string[];
  direction?: 'outgoing' | 'incoming' | 'both';
  weightProperty?: string;
}

export interface ShortestPathResponse {
  path?: GraphPath;
  distance: number;
  found: boolean;
  executionTime: number;
}

export interface PageRankRequest {
  iterations?: number;
  dampingFactor?: number;
  convergenceThreshold?: number;
  nodeLabels?: string[];
  relationshipTypes?: string[];
}

export interface PageRankResponse {
  scores: Record<UUID, number>;
  iterations: number;
  converged: boolean;
  executionTime: number;
}

export interface CommunityDetectionRequest {
  algorithm: 'louvain' | 'label_propagation' | 'connected_components';
  nodeLabels?: string[];
  relationshipTypes?: string[];
  minCommunitySize?: number;
}

export interface CommunityDetectionResponse {
  communities: Community[];
  modularity?: number;
  executionTime: number;
}

export interface Community {
  id: number;
  nodes: UUID[];
  size: number;
  density?: number;
}

export interface CentralityRequest {
  algorithm: 'betweenness' | 'closeness' | 'eigenvector' | 'degree';
  nodeLabels?: string[];
  relationshipTypes?: string[];
  direction?: 'outgoing' | 'incoming' | 'both';
  normalized?: boolean;
}

export interface CentralityResponse {
  scores: Record<UUID, number>;
  executionTime: number;
  statistics: {
    mean: number;
    median: number;
    stdDev: number;
    min: number;
    max: number;
  };
}

export interface GraphStatistics {
  nodeCount: number;
  edgeCount: number;
  labelDistribution: Record<string, number>;
  relationshipTypeDistribution: Record<string, number>;
  averageDegree: number;
  density: number;
  diameter?: number;
  averagePathLength?: number;
  clusteringCoefficient?: number;
  timestamp: Timestamp;
}

export interface GraphSchema {
  nodeLabels: NodeLabelSchema[];
  relationshipTypes: RelationshipTypeSchema[];
  constraints: GraphConstraint[];
  indexes: GraphIndex[];
}

export interface NodeLabelSchema {
  label: string;
  propertyKeys: string[];
  count: number;
}

export interface RelationshipTypeSchema {
  type: string;
  propertyKeys: string[];
  count: number;
  sourceLabels: string[];
  targetLabels: string[];
}

export interface GraphConstraint {
  id: UUID;
  type: 'unique' | 'exists' | 'node_key';
  nodeLabel?: string;
  relationshipType?: string;
  properties: string[];
  createdAt: Timestamp;
}

export interface GraphIndex {
  id: UUID;
  name: string;
  type: 'btree' | 'fulltext' | 'spatial';
  nodeLabel?: string;
  relationshipType?: string;
  properties: string[];
  unique: boolean;
  createdAt: Timestamp;
}

// ============================================================================
// Node Management APIs
// ============================================================================

/**
 * List graph nodes with optional filtering and pagination
 */
export async function listNodes(
  filters?: NodeFilters
): Promise<PaginatedResponse<GraphNode>> {
  const queryString = filters ? buildQueryParams(filters) : '';
  const response = await get<PaginatedResponse<GraphNode>>(
    `/graph/nodes${queryString}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch graph nodes');
  }

  return response.data;
}

/**
 * Create a new graph node
 */
export async function createNode(request: CreateNodeRequest): Promise<GraphNode> {
  const response = await post<GraphNode>('/graph/nodes', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create graph node');
  }

  return response.data;
}

/**
 * Get a graph node by ID
 */
export async function getNode(nodeId: UUID): Promise<GraphNode> {
  const response = await get<GraphNode>(`/graph/nodes/${nodeId}`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch graph node');
  }

  return response.data;
}

/**
 * Update a graph node
 */
export async function updateNode(
  nodeId: UUID,
  request: UpdateNodeRequest
): Promise<GraphNode> {
  const response = await patch<GraphNode>(`/graph/nodes/${nodeId}`, request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to update graph node');
  }

  return response.data;
}

/**
 * Delete a graph node
 */
export async function deleteNode(nodeId: UUID, cascade: boolean = false): Promise<void> {
  const response = await del<void>(
    `/graph/nodes/${nodeId}${buildQueryParams({ cascade })}`
  );

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete graph node');
  }
}

/**
 * Get neighbors of a node
 */
export async function getNodeNeighbors(
  nodeId: UUID,
  direction: 'outgoing' | 'incoming' | 'both' = 'both',
  relationshipTypes?: string[]
): Promise<GraphNode[]> {
  const params: Record<string, unknown> = { direction };
  if (relationshipTypes && relationshipTypes.length > 0) {
    params.types = relationshipTypes.join(',');
  }

  const response = await get<GraphNode[]>(
    `/graph/nodes/${nodeId}/neighbors${buildQueryParams(params)}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch node neighbors');
  }

  return response.data;
}

// ============================================================================
// Edge Management APIs
// ============================================================================

/**
 * List graph edges with optional filtering and pagination
 */
export async function listEdges(
  filters?: EdgeFilters
): Promise<PaginatedResponse<GraphEdge>> {
  const queryString = filters ? buildQueryParams(filters) : '';
  const response = await get<PaginatedResponse<GraphEdge>>(
    `/graph/edges${queryString}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch graph edges');
  }

  return response.data;
}

/**
 * Create a new graph edge
 */
export async function createEdge(request: CreateEdgeRequest): Promise<GraphEdge> {
  const response = await post<GraphEdge>('/graph/edges', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create graph edge');
  }

  return response.data;
}

/**
 * Get a graph edge by ID
 */
export async function getEdge(edgeId: UUID): Promise<GraphEdge> {
  const response = await get<GraphEdge>(`/graph/edges/${edgeId}`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch graph edge');
  }

  return response.data;
}

/**
 * Update a graph edge
 */
export async function updateEdge(
  edgeId: UUID,
  request: UpdateEdgeRequest
): Promise<GraphEdge> {
  const response = await patch<GraphEdge>(`/graph/edges/${edgeId}`, request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to update graph edge');
  }

  return response.data;
}

/**
 * Delete a graph edge
 */
export async function deleteEdge(edgeId: UUID): Promise<void> {
  const response = await del<void>(`/graph/edges/${edgeId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete graph edge');
  }
}

// ============================================================================
// Graph Query APIs
// ============================================================================

/**
 * Execute a PGQL-like graph query
 */
export async function executeQuery(
  request: GraphQueryRequest
): Promise<GraphQueryResponse> {
  const response = await post<GraphQueryResponse>('/graph/query', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to execute graph query');
  }

  return response.data;
}

/**
 * Explain a graph query execution plan
 */
export async function explainQuery(query: string): Promise<QueryExecutionPlan> {
  const response = await post<QueryExecutionPlan>('/graph/query/explain', {
    query,
  });

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to explain graph query');
  }

  return response.data;
}

// ============================================================================
// Graph Algorithm APIs
// ============================================================================

/**
 * Find shortest path between two nodes
 */
export async function findShortestPath(
  request: ShortestPathRequest
): Promise<ShortestPathResponse> {
  const response = await post<ShortestPathResponse>(
    '/graph/algorithms/shortest-path',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to find shortest path');
  }

  return response.data;
}

/**
 * Run PageRank algorithm
 */
export async function runPageRank(
  request?: PageRankRequest
): Promise<PageRankResponse> {
  const response = await post<PageRankResponse>(
    '/graph/algorithms/pagerank',
    request || {}
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to run PageRank');
  }

  return response.data;
}

/**
 * Detect communities in the graph
 */
export async function detectCommunities(
  request: CommunityDetectionRequest
): Promise<CommunityDetectionResponse> {
  const response = await post<CommunityDetectionResponse>(
    '/graph/algorithms/community',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to detect communities');
  }

  return response.data;
}

/**
 * Calculate centrality metrics
 */
export async function calculateCentrality(
  request: CentralityRequest
): Promise<CentralityResponse> {
  const response = await post<CentralityResponse>(
    '/graph/algorithms/centrality',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to calculate centrality');
  }

  return response.data;
}

// ============================================================================
// Graph Analytics APIs
// ============================================================================

/**
 * Get graph statistics
 */
export async function getGraphStatistics(): Promise<GraphStatistics> {
  const response = await get<GraphStatistics>('/graph/statistics');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch graph statistics');
  }

  return response.data;
}

/**
 * Get graph schema
 */
export async function getGraphSchema(): Promise<GraphSchema> {
  const response = await get<GraphSchema>('/graph/schema');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch graph schema');
  }

  return response.data;
}

/**
 * Create a graph constraint
 */
export async function createConstraint(
  constraint: Omit<GraphConstraint, 'id' | 'createdAt'>
): Promise<GraphConstraint> {
  const response = await post<GraphConstraint>('/graph/constraints', constraint);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create graph constraint');
  }

  return response.data;
}

/**
 * Delete a graph constraint
 */
export async function deleteConstraint(constraintId: UUID): Promise<void> {
  const response = await del<void>(`/graph/constraints/${constraintId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete graph constraint');
  }
}

/**
 * Create a graph index
 */
export async function createIndex(
  index: Omit<GraphIndex, 'id' | 'createdAt'>
): Promise<GraphIndex> {
  const response = await post<GraphIndex>('/graph/indexes', index);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create graph index');
  }

  return response.data;
}

/**
 * Delete a graph index
 */
export async function deleteIndex(indexId: UUID): Promise<void> {
  const response = await del<void>(`/graph/indexes/${indexId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete graph index');
  }
}

// ============================================================================
// Export Service Object (Alternative Pattern)
// ============================================================================

export const graphService = {
  // Nodes
  listNodes,
  createNode,
  getNode,
  updateNode,
  deleteNode,
  getNodeNeighbors,

  // Edges
  listEdges,
  createEdge,
  getEdge,
  updateEdge,
  deleteEdge,

  // Queries
  executeQuery,
  explainQuery,

  // Algorithms
  findShortestPath,
  runPageRank,
  detectCommunities,
  calculateCentrality,

  // Analytics
  getGraphStatistics,
  getGraphSchema,
  createConstraint,
  deleteConstraint,
  createIndex,
  deleteIndex,
};
