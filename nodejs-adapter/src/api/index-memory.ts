/**
 * Index & Memory Management API Client
 *
 * Provides TypeScript client methods for RustyDB index and memory management endpoints.
 * Includes index CRUD, statistics, advisor, memory allocator, buffer pool, and SIMD configuration.
 *
 * @module api/index-memory
 */

// ============================================================================
// Type Definitions
// ============================================================================

/**
 * Index information
 */
export interface IndexInfo {
  index_id: string;
  index_name: string;
  table_name: string;
  columns: string[];
  index_type: 'btree' | 'hash' | 'fulltext' | 'spatial' | 'bitmap' | 'lsm';
  is_unique: boolean;
  is_primary: boolean;
  size_bytes: number;
  row_count: number;
  created_at: number;
  last_used: number | null;
}

/**
 * Create index request
 */
export interface CreateIndexRequest {
  index_name: string;
  table_name: string;
  columns: string[];
  index_type?: 'btree' | 'hash' | 'fulltext' | 'spatial' | 'bitmap' | 'lsm';
  is_unique?: boolean;
  options?: Record<string, unknown>;
}

/**
 * Index statistics
 */
export interface IndexStats {
  index_id: string;
  index_name: string;
  scans: number;
  tuples_read: number;
  tuples_fetched: number;
  blocks_read: number;
  blocks_hit: number;
  hit_ratio: number;
  avg_scan_time_ms: number;
  last_scan: number | null;
}

/**
 * Index usage statistics
 */
export interface IndexUsageStats {
  total_indexes: number;
  used_indexes: number;
  unused_indexes: number;
  total_scans: number;
  total_tuples_read: number;
  most_used: IndexStats[];
  least_used: IndexStats[];
}

/**
 * Index advisor recommendation
 */
export interface IndexRecommendation {
  recommendation_id: string;
  table_name: string;
  suggested_columns: string[];
  index_type: string;
  reason: string;
  estimated_improvement: number;
  estimated_size_bytes: number;
  priority: 'high' | 'medium' | 'low';
  workload_queries: string[];
}

/**
 * Workload analysis request
 */
export interface AnalyzeWorkloadRequest {
  queries: string[];
  time_range_hours?: number;
  include_existing?: boolean;
}

/**
 * Memory allocator statistics
 */
export interface AllocatorStats {
  total_allocated: number;
  total_freed: number;
  current_usage: number;
  peak_usage: number;
  allocation_count: number;
  free_count: number;
  fragmentation_ratio: number;
  slab_stats: SlabStats[];
  arena_stats: ArenaStats[];
}

/**
 * Slab allocator statistics
 */
export interface SlabStats {
  slab_size: number;
  total_slabs: number;
  used_slabs: number;
  free_slabs: number;
  hit_ratio: number;
}

/**
 * Arena allocator statistics
 */
export interface ArenaStats {
  arena_id: string;
  total_size: number;
  used_size: number;
  allocation_count: number;
}

/**
 * Memory zone information
 */
export interface MemoryZoneInfo {
  zone_id: string;
  zone_name: string;
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  usage_percent: number;
  page_size: number;
  page_count: number;
}

/**
 * Memory pressure status
 */
export interface MemoryPressureStatus {
  pressure_level: 'none' | 'low' | 'medium' | 'high' | 'critical';
  memory_used_percent: number;
  available_memory_bytes: number;
  swap_used_percent: number;
  oom_risk: boolean;
  recommendations: string[];
  timestamp: number;
}

/**
 * Buffer pool configuration
 */
export interface BufferPoolConfig {
  size_bytes: number;
  page_size: number;
  eviction_policy: 'lru' | 'clock' | '2q' | 'lru_k' | 'lirs' | 'arc';
  prefetch_enabled: boolean;
  prefetch_lookahead: number;
  warmup_enabled: boolean;
}

/**
 * Buffer pool resize request
 */
export interface BufferPoolResizeRequest {
  new_size_bytes: number;
  immediate?: boolean;
}

/**
 * Eviction policy update request
 */
export interface EvictionPolicyRequest {
  policy: 'lru' | 'clock' | '2q' | 'lru_k' | 'lirs' | 'arc';
  parameters?: Record<string, unknown>;
}

/**
 * SIMD status
 */
export interface SimdStatus {
  available: boolean;
  enabled: boolean;
  features: SimdFeature[];
  benchmark_results: SimdBenchmark[];
}

/**
 * SIMD feature
 */
export interface SimdFeature {
  name: string;
  available: boolean;
  enabled: boolean;
  description: string;
}

/**
 * SIMD benchmark result
 */
export interface SimdBenchmark {
  operation: string;
  simd_time_ns: number;
  scalar_time_ns: number;
  speedup: number;
}

/**
 * SIMD configuration request
 */
export interface SimdConfigRequest {
  enable_avx2?: boolean;
  enable_avx512?: boolean;
  enable_neon?: boolean;
  auto_detect?: boolean;
}

// ============================================================================
// Client Configuration
// ============================================================================

export interface IndexMemoryClientConfig {
  baseUrl: string;
  apiVersion?: string;
  timeout?: number;
  headers?: Record<string, string>;
}

// ============================================================================
// Index & Memory Client
// ============================================================================

/**
 * Index & Memory Management API Client
 */
export class IndexMemoryClient {
  private baseUrl: string;
  private apiVersion: string;
  private timeout: number;
  private headers: Record<string, string>;

  constructor(config: IndexMemoryClientConfig) {
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
  // Index Operations
  // ============================================================================

  /** List all indexes */
  async listIndexes(tableName?: string): Promise<IndexInfo[]> {
    const path = tableName ? `/indexes?table=${encodeURIComponent(tableName)}` : '/indexes';
    return this.request<IndexInfo[]>('GET', path);
  }

  /** Get index details */
  async getIndex(indexId: string): Promise<IndexInfo> {
    return this.request<IndexInfo>('GET', `/indexes/${indexId}`);
  }

  /** Create a new index */
  async createIndex(request: CreateIndexRequest): Promise<IndexInfo> {
    return this.request<IndexInfo>('POST', '/indexes', request);
  }

  /** Drop an index */
  async dropIndex(indexId: string): Promise<void> {
    return this.request<void>('DELETE', `/indexes/${indexId}`);
  }

  /** Rebuild an index */
  async rebuildIndex(indexId: string, online?: boolean): Promise<{ status: string; duration_ms: number }> {
    return this.request<{ status: string; duration_ms: number }>('POST', `/indexes/${indexId}/rebuild`, { online });
  }

  /** Get index statistics */
  async getIndexStats(indexId: string): Promise<IndexStats> {
    return this.request<IndexStats>('GET', `/indexes/${indexId}/stats`);
  }

  /** Get overall index usage statistics */
  async getIndexUsageStats(): Promise<IndexUsageStats> {
    return this.request<IndexUsageStats>('GET', '/indexes/usage-stats');
  }

  // ============================================================================
  // Index Advisor
  // ============================================================================

  /** Analyze workload and get recommendations */
  async analyzeWorkload(request: AnalyzeWorkloadRequest): Promise<IndexRecommendation[]> {
    return this.request<IndexRecommendation[]>('POST', '/indexes/advisor/analyze', request);
  }

  /** Get cached recommendations */
  async getRecommendations(): Promise<IndexRecommendation[]> {
    return this.request<IndexRecommendation[]>('GET', '/indexes/advisor/recommendations');
  }

  /** Apply a recommendation */
  async applyRecommendation(recommendationId: string): Promise<IndexInfo> {
    return this.request<IndexInfo>('POST', `/indexes/advisor/recommendations/${recommendationId}/apply`);
  }

  /** Dismiss a recommendation */
  async dismissRecommendation(recommendationId: string): Promise<void> {
    return this.request<void>('DELETE', `/indexes/advisor/recommendations/${recommendationId}`);
  }

  // ============================================================================
  // Memory Allocator
  // ============================================================================

  /** Get allocator statistics */
  async getAllocatorStats(): Promise<AllocatorStats> {
    return this.request<AllocatorStats>('GET', '/memory/allocator/stats');
  }

  /** Get memory zone information */
  async getMemoryZones(): Promise<MemoryZoneInfo[]> {
    return this.request<MemoryZoneInfo[]>('GET', '/memory/zones');
  }

  /** Get memory pressure status */
  async getMemoryPressure(): Promise<MemoryPressureStatus> {
    return this.request<MemoryPressureStatus>('GET', '/memory/pressure');
  }

  /** Trigger memory compaction */
  async compactMemory(): Promise<{ freed_bytes: number; duration_ms: number }> {
    return this.request<{ freed_bytes: number; duration_ms: number }>('POST', '/memory/compact');
  }

  /** Clear memory caches */
  async clearCaches(cacheType?: string): Promise<{ cleared_bytes: number }> {
    return this.request<{ cleared_bytes: number }>('POST', '/memory/clear-caches', { cache_type: cacheType });
  }

  // ============================================================================
  // Buffer Pool Management
  // ============================================================================

  /** Get buffer pool configuration */
  async getBufferPoolConfig(): Promise<BufferPoolConfig> {
    return this.request<BufferPoolConfig>('GET', '/buffer-pool/config');
  }

  /** Resize buffer pool */
  async resizeBufferPool(request: BufferPoolResizeRequest): Promise<{ status: string; new_size: number }> {
    return this.request<{ status: string; new_size: number }>('POST', '/buffer-pool/resize', request);
  }

  /** Update eviction policy */
  async setEvictionPolicy(request: EvictionPolicyRequest): Promise<{ status: string }> {
    return this.request<{ status: string }>('POST', '/buffer-pool/eviction-policy', request);
  }

  /** Warmup buffer pool */
  async warmupBufferPool(tables?: string[]): Promise<{ pages_loaded: number; duration_ms: number }> {
    return this.request<{ pages_loaded: number; duration_ms: number }>('POST', '/buffer-pool/warmup', { tables });
  }

  /** Get buffer pool page contents */
  async getBufferPoolPages(limit?: number): Promise<{ pages: Array<{ page_id: string; table: string; dirty: boolean }> }> {
    const path = limit ? `/buffer-pool/pages?limit=${limit}` : '/buffer-pool/pages';
    return this.request<{ pages: Array<{ page_id: string; table: string; dirty: boolean }> }>('GET', path);
  }

  // ============================================================================
  // SIMD Configuration
  // ============================================================================

  /** Get SIMD status */
  async getSimdStatus(): Promise<SimdStatus> {
    return this.request<SimdStatus>('GET', '/simd/status');
  }

  /** Configure SIMD */
  async configureSimd(config: SimdConfigRequest): Promise<SimdStatus> {
    return this.request<SimdStatus>('POST', '/simd/configure', config);
  }

  /** Run SIMD benchmarks */
  async runSimdBenchmarks(): Promise<SimdBenchmark[]> {
    return this.request<SimdBenchmark[]>('POST', '/simd/benchmark');
  }

  /** Enable AVX2 */
  async enableAvx2(): Promise<{ enabled: boolean }> {
    return this.request<{ enabled: boolean }>('POST', '/simd/enable-avx2');
  }

  /** Enable AVX-512 */
  async enableAvx512(): Promise<{ enabled: boolean }> {
    return this.request<{ enabled: boolean }>('POST', '/simd/enable-avx512');
  }
}

// ============================================================================
// Factory Function
// ============================================================================

export function createIndexMemoryClient(config: IndexMemoryClientConfig): IndexMemoryClient {
  return new IndexMemoryClient(config);
}

export default IndexMemoryClient;
