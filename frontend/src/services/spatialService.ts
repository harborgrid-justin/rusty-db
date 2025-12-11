// ============================================================================
// Geospatial Service
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

export interface SpatialLayer {
  id: UUID;
  name: string;
  type: SpatialLayerType;
  geometryType: GeometryType;
  srid: number;
  extent?: BoundingBox;
  featureCount: number;
  properties: string[];
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export type SpatialLayerType = 'vector' | 'raster' | 'point_cloud';

export type GeometryType =
  | 'Point'
  | 'LineString'
  | 'Polygon'
  | 'MultiPoint'
  | 'MultiLineString'
  | 'MultiPolygon'
  | 'GeometryCollection';

export interface BoundingBox {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
  srid: number;
}

export interface Geometry {
  type: GeometryType;
  coordinates: number[] | number[][] | number[][][];
  srid?: number;
}

export interface Feature {
  id: UUID;
  layerId: UUID;
  geometry: Geometry;
  properties: Record<string, unknown>;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface CreateLayerRequest {
  name: string;
  type: SpatialLayerType;
  geometryType: GeometryType;
  srid?: number;
  properties?: string[];
}

export interface UpdateLayerRequest {
  name?: string;
  properties?: string[];
}

export interface CreateFeatureRequest {
  layerId: UUID;
  geometry: Geometry;
  properties?: Record<string, unknown>;
}

export interface UpdateFeatureRequest {
  geometry?: Geometry;
  properties?: Record<string, unknown>;
}

export interface BoundingBoxQueryRequest {
  layerId: UUID;
  bbox: BoundingBox;
  properties?: string[];
  limit?: number;
}

export interface BoundingBoxQueryResponse {
  features: Feature[];
  count: number;
  extent?: BoundingBox;
  executionTime: number;
}

export interface RadiusQueryRequest {
  layerId: UUID;
  center: {
    x: number;
    y: number;
    srid?: number;
  };
  radius: number;
  unit?: 'meters' | 'kilometers' | 'miles' | 'feet';
  properties?: string[];
  limit?: number;
}

export interface RadiusQueryResponse {
  features: FeatureWithDistance[];
  count: number;
  executionTime: number;
}

export interface FeatureWithDistance extends Feature {
  distance: number;
  unit: string;
}

export interface PolygonQueryRequest {
  layerId: UUID;
  polygon: Geometry;
  operation?: 'intersects' | 'within' | 'contains' | 'overlaps';
  properties?: string[];
  limit?: number;
}

export interface PolygonQueryResponse {
  features: Feature[];
  count: number;
  executionTime: number;
}

export interface SpatialIndex {
  id: UUID;
  name: string;
  layerId: UUID;
  type: SpatialIndexType;
  geometryColumn: string;
  createdAt: Timestamp;
  statistics?: {
    nodeCount: number;
    depth: number;
    coverage: number;
  };
}

export type SpatialIndexType = 'rtree' | 'quadtree' | 'geohash' | 'h3';

export interface CreateSpatialIndexRequest {
  name: string;
  layerId: UUID;
  type: SpatialIndexType;
  geometryColumn?: string;
}

export interface RoutingRequest {
  startPoint: {
    x: number;
    y: number;
    srid?: number;
  };
  endPoint: {
    x: number;
    y: number;
    srid?: number;
  };
  algorithm?: 'dijkstra' | 'astar' | 'bidirectional';
  costFunction?: 'distance' | 'time' | 'custom';
  constraints?: {
    maxDistance?: number;
    avoidAreas?: Geometry[];
    preferredAreas?: Geometry[];
  };
}

export interface RoutingResponse {
  route: {
    path: Geometry;
    distance: number;
    duration?: number;
    waypoints: RouteWaypoint[];
  };
  alternativeRoutes?: {
    path: Geometry;
    distance: number;
    duration?: number;
  }[];
  executionTime: number;
}

export interface RouteWaypoint {
  geometry: Geometry;
  instruction?: string;
  distance: number;
  duration?: number;
}

export interface SpatialAnalysisRequest {
  operation: SpatialOperation;
  geometries: Geometry[];
  parameters?: Record<string, unknown>;
}

export type SpatialOperation =
  | 'buffer'
  | 'intersection'
  | 'union'
  | 'difference'
  | 'convex_hull'
  | 'centroid'
  | 'simplify'
  | 'dissolve';

export interface SpatialAnalysisResponse {
  result: Geometry | Geometry[];
  area?: number;
  length?: number;
  executionTime: number;
}

export interface SpatialStatistics {
  layerId: UUID;
  featureCount: number;
  extent: BoundingBox;
  area?: number;
  perimeter?: number;
  densityPerSqKm?: number;
  averageFeatureSize?: number;
  geometryTypeDistribution: Record<GeometryType, number>;
  timestamp: Timestamp;
}

export interface HeatmapRequest {
  layerId: UUID;
  extent: BoundingBox;
  resolution: number;
  radius?: number;
  weightProperty?: string;
}

export interface HeatmapResponse {
  grid: number[][];
  extent: BoundingBox;
  resolution: number;
  minValue: number;
  maxValue: number;
  executionTime: number;
}

export interface GeocodingRequest {
  address: string;
  bounds?: BoundingBox;
  limit?: number;
}

export interface GeocodingResponse {
  results: GeocodingResult[];
  executionTime: number;
}

export interface GeocodingResult {
  address: string;
  geometry: Geometry;
  confidence: number;
  components: {
    street?: string;
    city?: string;
    state?: string;
    country?: string;
    postalCode?: string;
  };
}

export interface ReverseGeocodingRequest {
  point: {
    x: number;
    y: number;
    srid?: number;
  };
  radius?: number;
}

export interface LayerFilters extends PaginationParams {
  type?: SpatialLayerType;
  geometryType?: GeometryType;
  search?: string;
}

// ============================================================================
// Layer Management APIs
// ============================================================================

/**
 * List spatial layers with optional filtering and pagination
 */
export async function listLayers(
  filters?: LayerFilters
): Promise<PaginatedResponse<SpatialLayer>> {
  const queryString = filters ? buildQueryParams(filters) : '';
  const response = await get<PaginatedResponse<SpatialLayer>>(
    `/spatial/layers${queryString}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch spatial layers');
  }

  return response.data;
}

/**
 * Create a new spatial layer
 */
export async function createLayer(
  request: CreateLayerRequest
): Promise<SpatialLayer> {
  const response = await post<SpatialLayer>('/spatial/layers', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create spatial layer');
  }

  return response.data;
}

/**
 * Get a spatial layer by ID
 */
export async function getLayer(layerId: UUID): Promise<SpatialLayer> {
  const response = await get<SpatialLayer>(`/spatial/layers/${layerId}`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch spatial layer');
  }

  return response.data;
}

/**
 * Update a spatial layer
 */
export async function updateLayer(
  layerId: UUID,
  request: UpdateLayerRequest
): Promise<SpatialLayer> {
  const response = await patch<SpatialLayer>(
    `/spatial/layers/${layerId}`,
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to update spatial layer');
  }

  return response.data;
}

/**
 * Delete a spatial layer
 */
export async function deleteLayer(layerId: UUID): Promise<void> {
  const response = await del<void>(`/spatial/layers/${layerId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete spatial layer');
  }
}

// ============================================================================
// Feature Management APIs
// ============================================================================

/**
 * Create a new feature in a layer
 */
export async function createFeature(
  request: CreateFeatureRequest
): Promise<Feature> {
  const response = await post<Feature>('/spatial/features', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create feature');
  }

  return response.data;
}

/**
 * Get a feature by ID
 */
export async function getFeature(featureId: UUID): Promise<Feature> {
  const response = await get<Feature>(`/spatial/features/${featureId}`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch feature');
  }

  return response.data;
}

/**
 * Update a feature
 */
export async function updateFeature(
  featureId: UUID,
  request: UpdateFeatureRequest
): Promise<Feature> {
  const response = await patch<Feature>(
    `/spatial/features/${featureId}`,
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to update feature');
  }

  return response.data;
}

/**
 * Delete a feature
 */
export async function deleteFeature(featureId: UUID): Promise<void> {
  const response = await del<void>(`/spatial/features/${featureId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete feature');
  }
}

// ============================================================================
// Spatial Query APIs
// ============================================================================

/**
 * Query features within a bounding box
 */
export async function queryByBoundingBox(
  request: BoundingBoxQueryRequest
): Promise<BoundingBoxQueryResponse> {
  const response = await post<BoundingBoxQueryResponse>(
    '/spatial/query/bbox',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to query by bounding box');
  }

  return response.data;
}

/**
 * Query features within a radius
 */
export async function queryByRadius(
  request: RadiusQueryRequest
): Promise<RadiusQueryResponse> {
  const response = await post<RadiusQueryResponse>(
    '/spatial/query/radius',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to query by radius');
  }

  return response.data;
}

/**
 * Query features by polygon intersection
 */
export async function queryByPolygon(
  request: PolygonQueryRequest
): Promise<PolygonQueryResponse> {
  const response = await post<PolygonQueryResponse>(
    '/spatial/query/polygon',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to query by polygon');
  }

  return response.data;
}

// ============================================================================
// Spatial Index APIs
// ============================================================================

/**
 * List spatial indexes
 */
export async function listIndexes(): Promise<SpatialIndex[]> {
  const response = await get<SpatialIndex[]>('/spatial/indexes');

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch spatial indexes');
  }

  return response.data;
}

/**
 * Create a spatial index
 */
export async function createSpatialIndex(
  request: CreateSpatialIndexRequest
): Promise<SpatialIndex> {
  const response = await post<SpatialIndex>('/spatial/indexes', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create spatial index');
  }

  return response.data;
}

/**
 * Delete a spatial index
 */
export async function deleteSpatialIndex(indexId: UUID): Promise<void> {
  const response = await del<void>(`/spatial/indexes/${indexId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete spatial index');
  }
}

/**
 * Rebuild a spatial index
 */
export async function rebuildSpatialIndex(indexId: UUID): Promise<SpatialIndex> {
  const response = await post<SpatialIndex>(
    `/spatial/indexes/${indexId}/rebuild`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to rebuild spatial index');
  }

  return response.data;
}

// ============================================================================
// Routing APIs
// ============================================================================

/**
 * Calculate route between two points
 */
export async function calculateRoute(
  request: RoutingRequest
): Promise<RoutingResponse> {
  const response = await post<RoutingResponse>('/spatial/routing', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to calculate route');
  }

  return response.data;
}

// ============================================================================
// Spatial Analysis APIs
// ============================================================================

/**
 * Perform spatial analysis operation
 */
export async function performSpatialAnalysis(
  request: SpatialAnalysisRequest
): Promise<SpatialAnalysisResponse> {
  const response = await post<SpatialAnalysisResponse>(
    '/spatial/analysis',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to perform spatial analysis');
  }

  return response.data;
}

/**
 * Get spatial statistics for a layer
 */
export async function getLayerStatistics(
  layerId: UUID
): Promise<SpatialStatistics> {
  const response = await get<SpatialStatistics>(
    `/spatial/layers/${layerId}/statistics`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch layer statistics');
  }

  return response.data;
}

/**
 * Generate heatmap from point data
 */
export async function generateHeatmap(
  request: HeatmapRequest
): Promise<HeatmapResponse> {
  const response = await post<HeatmapResponse>('/spatial/heatmap', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to generate heatmap');
  }

  return response.data;
}

// ============================================================================
// Geocoding APIs
// ============================================================================

/**
 * Geocode an address to coordinates
 */
export async function geocode(
  request: GeocodingRequest
): Promise<GeocodingResponse> {
  const response = await post<GeocodingResponse>('/spatial/geocode', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to geocode address');
  }

  return response.data;
}

/**
 * Reverse geocode coordinates to address
 */
export async function reverseGeocode(
  request: ReverseGeocodingRequest
): Promise<GeocodingResult> {
  const response = await post<GeocodingResult>(
    '/spatial/reverse-geocode',
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to reverse geocode');
  }

  return response.data;
}

// ============================================================================
// Export Service Object (Alternative Pattern)
// ============================================================================

export const spatialService = {
  // Layers
  listLayers,
  createLayer,
  getLayer,
  updateLayer,
  deleteLayer,

  // Features
  createFeature,
  getFeature,
  updateFeature,
  deleteFeature,

  // Queries
  queryByBoundingBox,
  queryByRadius,
  queryByPolygon,

  // Indexes
  listIndexes,
  createSpatialIndex,
  deleteSpatialIndex,
  rebuildSpatialIndex,

  // Routing
  calculateRoute,

  // Analysis
  performSpatialAnalysis,
  getLayerStatistics,
  generateHeatmap,

  // Geocoding
  geocode,
  reverseGeocode,
};
