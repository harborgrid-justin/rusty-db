/**
 * Enterprise & Spatial API Client
 *
 * Provides TypeScript client methods for RustyDB enterprise features and spatial operations.
 * Includes multi-tenant, blockchain, autonomous, CEP, and geospatial endpoints.
 *
 * @module api/enterprise-spatial
 */

// ============================================================================
// Multi-Tenant Types
// ============================================================================

export interface TenantInfo {
  tenant_id: string;
  name: string;
  status: 'active' | 'suspended' | 'provisioning' | 'deprovisioning';
  created_at: number;
  resource_limits: TenantResourceLimits;
  usage: TenantUsage;
  metadata: Record<string, unknown>;
}

export interface TenantResourceLimits {
  max_connections: number;
  max_storage_bytes: number;
  max_cpu_percent: number;
  max_memory_bytes: number;
  max_iops: number;
}

export interface TenantUsage {
  connections: number;
  storage_bytes: number;
  cpu_percent: number;
  memory_bytes: number;
  iops: number;
}

export interface CreateTenantRequest {
  name: string;
  resource_limits?: Partial<TenantResourceLimits>;
  metadata?: Record<string, unknown>;
  clone_from?: string;
}

export interface RelocateTenantRequest {
  target_node: string;
  maintain_connections?: boolean;
}

// ============================================================================
// Blockchain Types
// ============================================================================

export interface BlockchainInfo {
  chain_id: string;
  name: string;
  block_count: number;
  last_block_hash: string;
  last_block_time: number;
  status: 'active' | 'syncing' | 'stale';
}

export interface BlockInfo {
  block_hash: string;
  block_number: number;
  previous_hash: string;
  timestamp: number;
  transaction_count: number;
  merkle_root: string;
  signature: string;
}

export interface CreateChainRequest {
  name: string;
  algorithm?: 'sha256' | 'sha3' | 'blake3';
  retention_days?: number;
}

export interface AuditTrailEntry {
  entry_id: string;
  chain_id: string;
  block_number: number;
  timestamp: number;
  operation: string;
  table_name: string;
  row_id: string;
  user_id: string;
  data_hash: string;
  verified: boolean;
}

export interface VerifyBlockResponse {
  block_hash: string;
  valid: boolean;
  chain_valid: boolean;
  error?: string;
}

// ============================================================================
// Autonomous Types
// ============================================================================

export interface AutoTuneStatus {
  enabled: boolean;
  last_run: number | null;
  recommendations_applied: number;
  pending_recommendations: number;
  performance_improvement_percent: number;
  tuning_areas: TuningArea[];
}

export interface TuningArea {
  area: string;
  current_value: unknown;
  recommended_value: unknown;
  impact: 'high' | 'medium' | 'low';
  applied: boolean;
}

export interface AutoTuneRequest {
  areas?: string[];
  dry_run?: boolean;
  apply_immediately?: boolean;
}

export interface AutoIndexStatus {
  enabled: boolean;
  indexes_created: number;
  indexes_dropped: number;
  last_analysis: number | null;
  pending_operations: AutoIndexOperation[];
}

export interface AutoIndexOperation {
  operation_type: 'create' | 'drop' | 'rebuild';
  table_name: string;
  index_name: string;
  reason: string;
  estimated_impact: number;
}

export interface SelfHealStatus {
  enabled: boolean;
  incidents_detected: number;
  incidents_resolved: number;
  current_issues: SelfHealIssue[];
}

export interface SelfHealIssue {
  issue_id: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  component: string;
  description: string;
  detected_at: number;
  status: 'detected' | 'diagnosing' | 'healing' | 'resolved' | 'failed';
  resolution?: string;
}

// ============================================================================
// CEP (Complex Event Processing) Types
// ============================================================================

export interface CepRule {
  rule_id: string;
  name: string;
  pattern: string;
  window_seconds: number;
  action: string;
  enabled: boolean;
  created_at: number;
  matches_count: number;
}

export interface CreateCepRuleRequest {
  name: string;
  pattern: string;
  window_seconds: number;
  action: string;
  enabled?: boolean;
}

export interface ProcessEventsRequest {
  events: CepEvent[];
}

export interface CepEvent {
  event_type: string;
  timestamp: number;
  data: Record<string, unknown>;
}

export interface PatternMatchResult {
  rule_id: string;
  matched: boolean;
  matched_events: CepEvent[];
  action_triggered: boolean;
}

// ============================================================================
// Spatial Types
// ============================================================================

export interface Geometry {
  type: 'Point' | 'LineString' | 'Polygon' | 'MultiPoint' | 'MultiLineString' | 'MultiPolygon' | 'GeometryCollection';
  coordinates: unknown;
  srid?: number;
}

export interface SpatialQueryRequest {
  geometry: Geometry;
  table_name: string;
  geometry_column?: string;
  limit?: number;
}

export interface SpatialQueryResult {
  features: SpatialFeature[];
  count: number;
  bbox: [number, number, number, number];
}

export interface SpatialFeature {
  id: string;
  geometry: Geometry;
  properties: Record<string, unknown>;
}

export interface DistanceRequest {
  geometry1: Geometry;
  geometry2: Geometry;
  unit?: 'meters' | 'kilometers' | 'miles' | 'feet';
}

export interface BufferRequest {
  geometry: Geometry;
  distance: number;
  unit?: 'meters' | 'kilometers' | 'miles';
  segments?: number;
}

export interface NetworkAnalysisRequest {
  start_point: Geometry;
  end_point: Geometry;
  network_table: string;
  cost_column?: string;
  options?: NetworkOptions;
}

export interface NetworkOptions {
  avoid_tolls?: boolean;
  avoid_highways?: boolean;
  vehicle_type?: string;
}

export interface RouteResult {
  geometry: Geometry;
  distance: number;
  duration_seconds: number;
  steps: RouteStep[];
}

export interface RouteStep {
  instruction: string;
  distance: number;
  duration_seconds: number;
  geometry: Geometry;
}

export interface CoverageRequest {
  points: Geometry[];
  radius: number;
  unit?: 'meters' | 'kilometers';
}

export interface CoverageResult {
  coverage_geometry: Geometry;
  area_sq_meters: number;
  points_covered: number;
}

// ============================================================================
// Client Configuration
// ============================================================================

export interface EnterpriseSpatialClientConfig {
  baseUrl: string;
  apiVersion?: string;
  timeout?: number;
  headers?: Record<string, string>;
}

// ============================================================================
// Enterprise & Spatial Client
// ============================================================================

export class EnterpriseSpatialClient {
  private baseUrl: string;
  private apiVersion: string;
  private timeout: number;
  private headers: Record<string, string>;

  constructor(config: EnterpriseSpatialClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
    this.apiVersion = config.apiVersion || 'v1';
    this.timeout = config.timeout || 30000;
    this.headers = config.headers || {};
  }

  private buildUrl(path: string): string {
    return `${this.baseUrl}/api/${this.apiVersion}${path}`;
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = this.buildUrl(path);
    const options: RequestInit = {
      method,
      headers: { 'Content-Type': 'application/json', ...this.headers },
      signal: AbortSignal.timeout(this.timeout),
    };
    if (body !== undefined) {
      options.body = JSON.stringify(body);
    }
    const response = await fetch(url, options);
    if (!response.ok) {
      const error = await response.json().catch(() => ({ message: response.statusText }));
      throw new Error(`[${response.status}] ${error.message}`);
    }
    if (response.status === 204) {
      return undefined as T;
    }
    return response.json();
  }

  // ============================================================================
  // Multi-Tenant Operations
  // ============================================================================

  async listTenants(): Promise<TenantInfo[]> {
    return this.request<TenantInfo[]>('GET', '/tenants');
  }

  async getTenant(tenantId: string): Promise<TenantInfo> {
    return this.request<TenantInfo>('GET', `/tenants/${tenantId}`);
  }

  async createTenant(request: CreateTenantRequest): Promise<TenantInfo> {
    return this.request<TenantInfo>('POST', '/tenants', request);
  }

  async deleteTenant(tenantId: string): Promise<void> {
    return this.request<void>('DELETE', `/tenants/${tenantId}`);
  }

  async getTenantStats(tenantId: string): Promise<TenantUsage> {
    return this.request<TenantUsage>('GET', `/tenants/${tenantId}/stats`);
  }

  async relocateTenant(tenantId: string, request: RelocateTenantRequest): Promise<{ status: string }> {
    return this.request<{ status: string }>('POST', `/tenants/${tenantId}/relocate`, request);
  }

  async cloneTenant(tenantId: string, newName: string): Promise<TenantInfo> {
    return this.request<TenantInfo>('POST', `/tenants/${tenantId}/clone`, { new_name: newName });
  }

  // ============================================================================
  // Blockchain Operations
  // ============================================================================

  async listChains(): Promise<BlockchainInfo[]> {
    return this.request<BlockchainInfo[]>('GET', '/blockchain/chains');
  }

  async getChain(chainId: string): Promise<BlockchainInfo> {
    return this.request<BlockchainInfo>('GET', `/blockchain/chains/${chainId}`);
  }

  async createChain(request: CreateChainRequest): Promise<BlockchainInfo> {
    return this.request<BlockchainInfo>('POST', '/blockchain/chains', request);
  }

  async getBlock(chainId: string, blockNumber: number): Promise<BlockInfo> {
    return this.request<BlockInfo>('GET', `/blockchain/chains/${chainId}/blocks/${blockNumber}`);
  }

  async verifyBlock(chainId: string, blockNumber: number): Promise<VerifyBlockResponse> {
    return this.request<VerifyBlockResponse>('POST', `/blockchain/chains/${chainId}/blocks/${blockNumber}/verify`);
  }

  async getAuditTrail(tableName: string, rowId?: string): Promise<AuditTrailEntry[]> {
    const path = rowId
      ? `/blockchain/audit/${tableName}/${rowId}`
      : `/blockchain/audit/${tableName}`;
    return this.request<AuditTrailEntry[]>('GET', path);
  }

  // ============================================================================
  // Autonomous Operations
  // ============================================================================

  async getAutoTuneStatus(): Promise<AutoTuneStatus> {
    return this.request<AutoTuneStatus>('GET', '/autonomous/auto-tune/status');
  }

  async runAutoTune(request?: AutoTuneRequest): Promise<AutoTuneStatus> {
    return this.request<AutoTuneStatus>('POST', '/autonomous/auto-tune/run', request || {});
  }

  async getAutoIndexStatus(): Promise<AutoIndexStatus> {
    return this.request<AutoIndexStatus>('GET', '/autonomous/auto-index/status');
  }

  async enableAutoIndex(): Promise<{ enabled: boolean }> {
    return this.request<{ enabled: boolean }>('POST', '/autonomous/auto-index/enable');
  }

  async disableAutoIndex(): Promise<{ enabled: boolean }> {
    return this.request<{ enabled: boolean }>('POST', '/autonomous/auto-index/disable');
  }

  async getSelfHealStatus(): Promise<SelfHealStatus> {
    return this.request<SelfHealStatus>('GET', '/autonomous/self-heal/status');
  }

  async triggerSelfHeal(component?: string): Promise<SelfHealStatus> {
    return this.request<SelfHealStatus>('POST', '/autonomous/self-heal/trigger', { component });
  }

  // ============================================================================
  // CEP Operations
  // ============================================================================

  async listCepRules(): Promise<CepRule[]> {
    return this.request<CepRule[]>('GET', '/cep/rules');
  }

  async getCepRule(ruleId: string): Promise<CepRule> {
    return this.request<CepRule>('GET', `/cep/rules/${ruleId}`);
  }

  async createCepRule(request: CreateCepRuleRequest): Promise<CepRule> {
    return this.request<CepRule>('POST', '/cep/rules', request);
  }

  async deleteCepRule(ruleId: string): Promise<void> {
    return this.request<void>('DELETE', `/cep/rules/${ruleId}`);
  }

  async processEvents(request: ProcessEventsRequest): Promise<PatternMatchResult[]> {
    return this.request<PatternMatchResult[]>('POST', '/cep/process', request);
  }

  async testPattern(pattern: string, events: CepEvent[]): Promise<PatternMatchResult> {
    return this.request<PatternMatchResult>('POST', '/cep/test-pattern', { pattern, events });
  }

  // ============================================================================
  // Spatial Operations
  // ============================================================================

  async stContains(request: SpatialQueryRequest): Promise<SpatialQueryResult> {
    return this.request<SpatialQueryResult>('POST', '/spatial/contains', request);
  }

  async stIntersects(request: SpatialQueryRequest): Promise<SpatialQueryResult> {
    return this.request<SpatialQueryResult>('POST', '/spatial/intersects', request);
  }

  async stWithin(request: SpatialQueryRequest): Promise<SpatialQueryResult> {
    return this.request<SpatialQueryResult>('POST', '/spatial/within', request);
  }

  async stDistance(request: DistanceRequest): Promise<{ distance: number; unit: string }> {
    return this.request<{ distance: number; unit: string }>('POST', '/spatial/distance', request);
  }

  async stBuffer(request: BufferRequest): Promise<{ geometry: Geometry }> {
    return this.request<{ geometry: Geometry }>('POST', '/spatial/buffer', request);
  }

  async stUnion(geometries: Geometry[]): Promise<{ geometry: Geometry }> {
    return this.request<{ geometry: Geometry }>('POST', '/spatial/union', { geometries });
  }

  async stIntersection(geometry1: Geometry, geometry2: Geometry): Promise<{ geometry: Geometry }> {
    return this.request<{ geometry: Geometry }>('POST', '/spatial/intersection', { geometry1, geometry2 });
  }

  async stArea(geometry: Geometry, unit?: string): Promise<{ area: number; unit: string }> {
    return this.request<{ area: number; unit: string }>('POST', '/spatial/area', { geometry, unit });
  }

  async stLength(geometry: Geometry, unit?: string): Promise<{ length: number; unit: string }> {
    return this.request<{ length: number; unit: string }>('POST', '/spatial/length', { geometry, unit });
  }

  async stCentroid(geometry: Geometry): Promise<{ geometry: Geometry }> {
    return this.request<{ geometry: Geometry }>('POST', '/spatial/centroid', { geometry });
  }

  // ============================================================================
  // Network Analysis
  // ============================================================================

  async shortestPath(request: NetworkAnalysisRequest): Promise<RouteResult> {
    return this.request<RouteResult>('POST', '/spatial/network/shortest-path', request);
  }

  async routing(request: NetworkAnalysisRequest): Promise<RouteResult> {
    return this.request<RouteResult>('POST', '/spatial/network/route', request);
  }

  async serviceCoverage(request: CoverageRequest): Promise<CoverageResult> {
    return this.request<CoverageResult>('POST', '/spatial/network/coverage', request);
  }

  async nearestNeighbors(geometry: Geometry, tableName: string, k: number): Promise<SpatialFeature[]> {
    return this.request<SpatialFeature[]>('POST', '/spatial/nearest', { geometry, table_name: tableName, k });
  }
}

// ============================================================================
// Factory Function
// ============================================================================

export function createEnterpriseSpatialClient(config: EnterpriseSpatialClientConfig): EnterpriseSpatialClient {
  return new EnterpriseSpatialClient(config);
}

export default EnterpriseSpatialClient;
