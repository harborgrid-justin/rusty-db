/**
 * RustyDB Query & Optimizer API Adapter
 *
 * Provides comprehensive TypeScript/Node.js bindings for all query execution
 * and optimizer-related REST API endpoints in RustyDB.
 *
 * @module query-optimizer
 * @author Agent 9 - Query Processing & Optimization Specialist
 */

import axios, { AxiosInstance, AxiosError } from 'axios';

// ============================================================================
// Type Definitions - Query Execution
// ============================================================================

export interface QueryRequest {
  /** SQL query to execute */
  sql: string;
  /** Query parameters for parameterized queries */
  params?: unknown[];
  /** Maximum number of rows to return */
  limit?: number;
  /** Offset for pagination */
  offset?: number;
  /** Timeout in seconds */
  timeout?: number;
  /** Return query plan */
  explain?: boolean;
  /** Transaction ID (if part of transaction) */
  transaction_id?: number;
}

export interface ColumnMetadata {
  /** Column name */
  name: string;
  /** Data type (e.g., INTEGER, VARCHAR, TEXT) */
  data_type: string;
  /** Whether the column is nullable */
  nullable: boolean;
  /** Precision for numeric types */
  precision?: number;
  /** Scale for decimal types */
  scale?: number;
}

export interface QueryResponse {
  /** Unique query execution ID */
  query_id: string;
  /** Result rows as key-value objects */
  rows: Record<string, unknown>[];
  /** Column metadata */
  columns: ColumnMetadata[];
  /** Number of rows returned */
  row_count: number;
  /** Number of rows affected (INSERT/UPDATE/DELETE) */
  affected_rows?: number;
  /** Execution time in milliseconds */
  execution_time_ms: number;
  /** Query plan (if requested) */
  plan?: string;
  /** Any warnings generated during execution */
  warnings: string[];
  /** Whether there are more results available */
  has_more: boolean;
}

export interface BatchRequest {
  /** List of SQL statements to execute */
  statements: string[];
  /** Execute in transaction */
  transactional: boolean;
  /** Stop on first error */
  stop_on_error: boolean;
  /** Transaction isolation level */
  isolation?: string;
}

export interface BatchStatementResult {
  /** Index of statement in batch */
  statement_index: number;
  /** Whether statement succeeded */
  success: boolean;
  /** Number of rows affected */
  affected_rows?: number;
  /** Error message if failed */
  error?: string;
  /** Execution time in milliseconds */
  execution_time_ms: number;
}

export interface BatchResponse {
  /** Batch execution ID */
  batch_id: string;
  /** Results for each statement */
  results: BatchStatementResult[];
  /** Total execution time in milliseconds */
  total_time_ms: number;
  /** Number of successful statements */
  success_count: number;
  /** Number of failed statements */
  failure_count: number;
}

// ============================================================================
// Type Definitions - Query Explain
// ============================================================================

export interface ExplainRequest {
  /** SQL query to explain */
  query: string;
  /** Whether to actually execute and analyze */
  analyze?: boolean;
}

export interface ExplainPlan {
  /** Operator type (SeqScan, IndexScan, HashJoin, etc.) */
  operator: string;
  /** Estimated cost */
  cost: number;
  /** Estimated number of rows */
  rows: number;
  /** Additional operator-specific details */
  details: Record<string, unknown>;
  /** Child plan nodes */
  children: ExplainPlan[];
}

export interface ExplainResponse {
  /** Original query */
  query: string;
  /** Query execution plan tree */
  plan: ExplainPlan;
  /** Estimated total cost */
  estimated_cost: number;
  /** Estimated number of rows */
  estimated_rows: number;
  /** Time spent planning the query (ms) */
  planning_time_ms: number;
  /** Actual execution time (ms) - only for EXPLAIN ANALYZE */
  execution_time_ms?: number;
}

// ============================================================================
// Type Definitions - Optimizer Hints
// ============================================================================

export interface HintDefinition {
  /** Hint name (e.g., FULL, INDEX, HASH_JOIN) */
  name: string;
  /** Hint category (AccessPath, JoinMethod, etc.) */
  category: string;
  /** Description of what the hint does */
  description: string;
  /** List of required parameters */
  parameters: string[];
  /** Example usage */
  example: string;
}

export interface ListHintsQuery {
  /** Filter by category */
  category?: string;
  /** Search term for name/description */
  search?: string;
}

export interface HintsListResponse {
  /** List of available hints */
  hints: HintDefinition[];
  /** Total number of hints */
  total: number;
}

export interface HintInfo {
  /** Hint string */
  hint: string;
  /** When the hint was applied */
  applied_at: string;
  /** Whether the hint is currently effective */
  effective: boolean;
}

export interface ActiveHintsResponse {
  /** Session ID */
  session_id: string;
  /** Active hints in the session */
  hints: HintInfo[];
}

export interface ApplyHintRequest {
  /** SQL query */
  query: string;
  /** List of hints to apply */
  hints: string[];
}

export interface ApplyHintResponse {
  /** Unique hint application ID */
  hint_id: string;
  /** Successfully parsed hints */
  parsed_hints: string[];
  /** Any conflicting hints */
  conflicts: string[];
  /** Warnings about hint usage */
  warnings: string[];
}

// ============================================================================
// Type Definitions - Plan Baselines
// ============================================================================

export interface CreateBaselineRequest {
  /** SQL query text */
  query_text: string;
  /** Parameter types for query fingerprinting */
  param_types?: string[];
  /** Schema version */
  schema_version?: number;
  /** Whether baseline is enabled */
  enabled?: boolean;
  /** Whether baseline is fixed (cannot evolve) */
  fixed?: boolean;
}

export interface PlanSummary {
  /** Plan identifier */
  plan_id: number;
  /** Estimated cost */
  cost: number;
  /** Estimated cardinality */
  cardinality: number;
  /** Root operator type */
  operator_type: string;
  /** Whether plan is from baseline */
  from_baseline: boolean;
}

export interface BaselineResponse {
  /** Query fingerprint (unique identifier) */
  fingerprint: string;
  /** Whether baseline is enabled */
  enabled: boolean;
  /** Whether baseline is fixed */
  fixed: boolean;
  /** Origin of baseline (Manual, AutoCapture, Evolved) */
  origin: string;
  /** Creation timestamp */
  created_at: string;
  /** Last modification timestamp */
  last_modified: string;
  /** Last evolution timestamp */
  last_evolved?: string;
  /** Number of times query has been executed */
  execution_count: number;
  /** Average execution time in milliseconds */
  avg_execution_time_ms: number;
  /** Number of accepted plans in baseline */
  accepted_plans_count: number;
}

export interface BaselineDetailResponse {
  /** Query fingerprint (unique identifier) */
  fingerprint: string;
  /** Whether baseline is enabled */
  enabled: boolean;
  /** Whether baseline is fixed */
  fixed: boolean;
  /** Origin of baseline */
  origin: string;
  /** Creation timestamp */
  created_at: string;
  /** Last modification timestamp */
  last_modified: string;
  /** Last evolution timestamp */
  last_evolved?: string;
  /** Number of times query has been executed */
  execution_count: number;
  /** Average execution time in milliseconds */
  avg_execution_time_ms: number;
  /** List of accepted plans */
  accepted_plans: PlanSummary[];
}

export interface BaselinesListResponse {
  /** List of baselines */
  baselines: BaselineResponse[];
  /** Total number of baselines */
  total: number;
}

export interface UpdateBaselineRequest {
  /** Enable or disable the baseline */
  enabled?: boolean;
  /** Fix or unfix the baseline */
  fixed?: boolean;
}

export interface EvolveBaselineResponse {
  /** Number of plans evolved */
  evolved_plans: number;
  /** IDs of newly added plans */
  new_plans_added: number[];
  /** Time taken for evolution in milliseconds */
  evolution_time_ms: number;
}

// ============================================================================
// Type Definitions - Cost Model & Configuration
// ============================================================================

export interface OptimizerConfig {
  /** Enable cost-based optimization */
  cost_based: boolean;
  /** Enable adaptive query execution */
  adaptive_execution: boolean;
  /** Enable plan baselines */
  plan_baselines: boolean;
  /** Maximum optimization time in milliseconds */
  max_optimization_time_ms: number;
  /** Join reordering threshold */
  join_reorder_threshold: number;
}

export interface CostModel {
  /** CPU cost per tuple */
  cpu_tuple_cost: number;
  /** CPU cost per operator */
  cpu_operator_cost: number;
  /** Sequential page cost */
  seq_page_cost: number;
  /** Random page cost */
  random_page_cost: number;
  /** Effective cache size in pages */
  effective_cache_size: number;
}

// ============================================================================
// Error Types
// ============================================================================

export interface ApiError {
  /** Error code */
  code: string;
  /** Error message */
  message: string;
  /** Additional error details */
  details?: unknown;
  /** Error timestamp */
  timestamp: number;
  /** Request ID for tracing */
  request_id?: string;
}

export class QueryOptimizerError extends Error {
  constructor(
    public code: string,
    message: string,
    public details?: unknown
  ) {
    super(message);
    this.name = 'QueryOptimizerError';
  }

  static fromApiError(error: ApiError): QueryOptimizerError {
    return new QueryOptimizerError(error.code, error.message, error.details);
  }

  static fromAxiosError(error: AxiosError): QueryOptimizerError {
    if (error.response?.data) {
      const apiError = error.response.data as ApiError;
      return QueryOptimizerError.fromApiError(apiError);
    }
    return new QueryOptimizerError(
      'NETWORK_ERROR',
      error.message,
      { status: error.response?.status }
    );
  }
}

// ============================================================================
// Query & Optimizer API Client
// ============================================================================

export interface QueryOptimizerClientOptions {
  /** Base URL of RustyDB API (e.g., http://localhost:8080) */
  baseUrl: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** API key for authentication */
  apiKey?: string;
  /** Custom headers to include in all requests */
  headers?: Record<string, string>;
}

/**
 * RustyDB Query & Optimizer API Client
 *
 * Provides methods for:
 * - Executing SQL queries
 * - Analyzing query plans (EXPLAIN/EXPLAIN ANALYZE)
 * - Managing optimizer hints
 * - Managing SQL plan baselines
 * - Configuring query optimizer
 */
export class QueryOptimizerClient {
  private client: AxiosInstance;

  constructor(options: QueryOptimizerClientOptions) {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...options.headers,
    };

    if (options.apiKey) {
      headers['X-API-Key'] = options.apiKey;
    }

    this.client = axios.create({
      baseURL: options.baseUrl,
      timeout: options.timeout || 30000,
      headers,
    });
  }

  // ========================================================================
  // Query Execution Methods
  // ========================================================================

  /**
   * Execute a SQL query
   *
   * @param request - Query request parameters
   * @returns Query response with results
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const result = await client.executeQuery({
   *   sql: 'SELECT * FROM users WHERE age > $1',
   *   params: [18],
   *   limit: 100
   * });
   * console.log(`Retrieved ${result.row_count} rows in ${result.execution_time_ms}ms`);
   * ```
   */
  async executeQuery(request: QueryRequest): Promise<QueryResponse> {
    try {
      const response = await this.client.post<QueryResponse>(
        '/api/v1/query',
        request
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Execute multiple SQL statements in a batch
   *
   * @param request - Batch request parameters
   * @returns Batch response with individual statement results
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const result = await client.executeBatch({
   *   statements: [
   *     'INSERT INTO users (name) VALUES (\'Alice\')',
   *     'INSERT INTO users (name) VALUES (\'Bob\')',
   *     'UPDATE users SET active = true'
   *   ],
   *   transactional: true,
   *   stop_on_error: true
   * });
   * console.log(`${result.success_count} statements succeeded`);
   * ```
   */
  async executeBatch(request: BatchRequest): Promise<BatchResponse> {
    try {
      const response = await this.client.post<BatchResponse>(
        '/api/v1/batch',
        request
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  // ========================================================================
  // Query Explain Methods
  // ========================================================================

  /**
   * Get query execution plan (EXPLAIN)
   *
   * Shows the optimizer's chosen execution plan without actually running the query.
   *
   * @param query - SQL query to explain
   * @returns Explain response with plan tree and cost estimates
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const plan = await client.explainQuery('SELECT * FROM users WHERE age > 18');
   * console.log(`Estimated cost: ${plan.estimated_cost}`);
   * console.log(`Estimated rows: ${plan.estimated_rows}`);
   * console.log(`Root operator: ${plan.plan.operator}`);
   * ```
   */
  async explainQuery(query: string): Promise<ExplainResponse> {
    try {
      const response = await this.client.post<ExplainResponse>(
        '/api/v1/query/explain',
        { query, analyze: false }
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Get query execution plan with actual execution statistics (EXPLAIN ANALYZE)
   *
   * Executes the query and provides both estimated and actual execution metrics.
   *
   * @param query - SQL query to analyze
   * @returns Explain response with plan tree, cost estimates, and actual runtime
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const analysis = await client.explainAnalyzeQuery('SELECT * FROM users WHERE age > 18');
   * console.log(`Planning time: ${analysis.planning_time_ms}ms`);
   * console.log(`Execution time: ${analysis.execution_time_ms}ms`);
   * console.log(`Estimated cost: ${analysis.estimated_cost}`);
   * ```
   */
  async explainAnalyzeQuery(query: string): Promise<ExplainResponse> {
    try {
      const response = await this.client.post<ExplainResponse>(
        '/api/v1/query/explain/analyze',
        { query, analyze: true }
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Compare query plans with and without hints
   *
   * Utility method to analyze the impact of optimizer hints.
   *
   * @param query - SQL query to analyze
   * @param queryWithHints - Same query with hints applied
   * @returns Object with both plans for comparison
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const comparison = await client.comparePlans(
   *   'SELECT * FROM users WHERE age > 18',
   *   'SELECT /*+ FULL(users) * / * FROM users WHERE age > 18'
   * );
   * console.log(`Original cost: ${comparison.original.estimated_cost}`);
   * console.log(`With hints cost: ${comparison.withHints.estimated_cost}`);
   * ```
   */
  async comparePlans(
    query: string,
    queryWithHints: string
  ): Promise<{ original: ExplainResponse; withHints: ExplainResponse }> {
    const [original, withHints] = await Promise.all([
      this.explainQuery(query),
      this.explainQuery(queryWithHints),
    ]);
    return { original, withHints };
  }

  // ========================================================================
  // Optimizer Hints Methods
  // ========================================================================

  /**
   * List all available optimizer hints
   *
   * @param query - Optional filter parameters
   * @returns List of hint definitions
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * // List all hints
   * const allHints = await client.listHints();
   *
   * // Filter by category
   * const joinHints = await client.listHints({ category: 'JoinMethod' });
   *
   * // Search by keyword
   * const indexHints = await client.listHints({ search: 'index' });
   * ```
   */
  async listHints(query?: ListHintsQuery): Promise<HintsListResponse> {
    try {
      const response = await this.client.get<HintsListResponse>(
        '/api/v1/optimizer/hints',
        { params: query }
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Get active hints for the current session
   *
   * @returns Active hints response
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const activeHints = await client.getActiveHints();
   * console.log(`Session ${activeHints.session_id} has ${activeHints.hints.length} active hints`);
   * ```
   */
  async getActiveHints(): Promise<ActiveHintsResponse> {
    try {
      const response = await this.client.get<ActiveHintsResponse>(
        '/api/v1/optimizer/hints/active'
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Apply optimizer hints to a query
   *
   * @param request - Hint application request
   * @returns Result of hint application
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const result = await client.applyHints({
   *   query: 'SELECT * FROM users u JOIN orders o ON u.id = o.user_id',
   *   hints: ['HASH_JOIN(u o)', 'PARALLEL(4)']
   * });
   * console.log(`Applied hints: ${result.parsed_hints.join(', ')}`);
   * if (result.warnings.length > 0) {
   *   console.warn('Warnings:', result.warnings);
   * }
   * ```
   */
  async applyHints(request: ApplyHintRequest): Promise<ApplyHintResponse> {
    try {
      const response = await this.client.post<ApplyHintResponse>(
        '/api/v1/optimizer/hints',
        request
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Remove a specific hint by ID
   *
   * @param hintId - Hint ID to remove
   * @returns Success response
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * await client.removeHint('hint_abc123');
   * console.log('Hint removed successfully');
   * ```
   */
  async removeHint(hintId: string): Promise<{ message: string; success: boolean }> {
    try {
      const response = await this.client.delete<{ message: string; success: boolean }>(
        `/api/v1/optimizer/hints/${hintId}`
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  // ========================================================================
  // Plan Baselines Methods
  // ========================================================================

  /**
   * List all SQL plan baselines
   *
   * @returns List of baselines
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const baselines = await client.listBaselines();
   * console.log(`Found ${baselines.total} baselines`);
   * baselines.baselines.forEach(b => {
   *   console.log(`${b.fingerprint}: ${b.execution_count} executions, avg ${b.avg_execution_time_ms}ms`);
   * });
   * ```
   */
  async listBaselines(): Promise<BaselinesListResponse> {
    try {
      const response = await this.client.get<BaselinesListResponse>(
        '/api/v1/optimizer/baselines'
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Create a new SQL plan baseline
   *
   * Captures the current optimal plan for a query to ensure plan stability.
   *
   * @param request - Baseline creation request
   * @returns Created baseline
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const baseline = await client.createBaseline({
   *   query_text: 'SELECT * FROM users WHERE age > $1',
   *   param_types: ['INTEGER'],
   *   enabled: true,
   *   fixed: false
   * });
   * console.log(`Created baseline ${baseline.fingerprint}`);
   * ```
   */
  async createBaseline(request: CreateBaselineRequest): Promise<BaselineResponse> {
    try {
      const response = await this.client.post<BaselineResponse>(
        '/api/v1/optimizer/baselines',
        request
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Get detailed information about a specific plan baseline
   *
   * @param fingerprint - Query fingerprint (baseline ID)
   * @returns Baseline details including all accepted plans
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const baseline = await client.getBaseline('query_fingerprint_123');
   * console.log(`Baseline has ${baseline.accepted_plans.length} accepted plans`);
   * baseline.accepted_plans.forEach(plan => {
   *   console.log(`Plan ${plan.plan_id}: cost=${plan.cost}, rows=${plan.cardinality}`);
   * });
   * ```
   */
  async getBaseline(fingerprint: string): Promise<BaselineDetailResponse> {
    try {
      const response = await this.client.get<BaselineDetailResponse>(
        `/api/v1/optimizer/baselines/${fingerprint}`
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Update plan baseline settings
   *
   * @param fingerprint - Query fingerprint (baseline ID)
   * @param request - Update parameters
   * @returns Success response
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * // Disable a baseline
   * await client.updateBaseline('query_fingerprint_123', { enabled: false });
   *
   * // Fix a baseline to prevent evolution
   * await client.updateBaseline('query_fingerprint_123', { fixed: true });
   * ```
   */
  async updateBaseline(
    fingerprint: string,
    request: UpdateBaselineRequest
  ): Promise<{ message: string; fingerprint: string }> {
    try {
      const response = await this.client.put<{ message: string; fingerprint: string }>(
        `/api/v1/optimizer/baselines/${fingerprint}`,
        request
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Delete a plan baseline
   *
   * @param fingerprint - Query fingerprint (baseline ID)
   * @returns Success response
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * await client.deleteBaseline('query_fingerprint_123');
   * console.log('Baseline deleted');
   * ```
   */
  async deleteBaseline(fingerprint: string): Promise<{ message: string; fingerprint: string }> {
    try {
      const response = await this.client.delete<{ message: string; fingerprint: string }>(
        `/api/v1/optimizer/baselines/${fingerprint}`
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  /**
   * Evolve a plan baseline with new candidate plans
   *
   * Allows the optimizer to add new, potentially better plans to a baseline.
   *
   * @param fingerprint - Query fingerprint (baseline ID)
   * @returns Evolution results
   * @throws QueryOptimizerError on failure
   *
   * @example
   * ```typescript
   * const result = await client.evolveBaseline('query_fingerprint_123');
   * console.log(`Evolved ${result.evolved_plans} plans in ${result.evolution_time_ms}ms`);
   * console.log(`Added ${result.new_plans_added.length} new plans`);
   * ```
   */
  async evolveBaseline(fingerprint: string): Promise<EvolveBaselineResponse> {
    try {
      const response = await this.client.post<EvolveBaselineResponse>(
        `/api/v1/optimizer/baselines/${fingerprint}/evolve`
      );
      return response.data;
    } catch (error) {
      throw QueryOptimizerError.fromAxiosError(error as AxiosError);
    }
  }

  // ========================================================================
  // Utility Methods
  // ========================================================================

  /**
   * Enable a baseline
   *
   * Convenience method to enable a baseline.
   *
   * @param fingerprint - Query fingerprint
   * @returns Success response
   */
  async enableBaseline(fingerprint: string): Promise<{ message: string; fingerprint: string }> {
    return this.updateBaseline(fingerprint, { enabled: true });
  }

  /**
   * Disable a baseline
   *
   * Convenience method to disable a baseline.
   *
   * @param fingerprint - Query fingerprint
   * @returns Success response
   */
  async disableBaseline(fingerprint: string): Promise<{ message: string; fingerprint: string }> {
    return this.updateBaseline(fingerprint, { enabled: false });
  }

  /**
   * Fix a baseline (prevent evolution)
   *
   * Convenience method to fix a baseline.
   *
   * @param fingerprint - Query fingerprint
   * @returns Success response
   */
  async fixBaseline(fingerprint: string): Promise<{ message: string; fingerprint: string }> {
    return this.updateBaseline(fingerprint, { fixed: true });
  }

  /**
   * Unfix a baseline (allow evolution)
   *
   * Convenience method to unfix a baseline.
   *
   * @param fingerprint - Query fingerprint
   * @returns Success response
   */
  async unfixBaseline(fingerprint: string): Promise<{ message: string; fingerprint: string }> {
    return this.updateBaseline(fingerprint, { fixed: false });
  }

  /**
   * Get query statistics from baseline
   *
   * Extract execution statistics from a baseline.
   *
   * @param fingerprint - Query fingerprint
   * @returns Execution statistics
   */
  async getBaselineStats(fingerprint: string): Promise<{
    execution_count: number;
    avg_execution_time_ms: number;
    plan_count: number;
  }> {
    const baseline = await this.getBaseline(fingerprint);
    return {
      execution_count: baseline.execution_count,
      avg_execution_time_ms: baseline.avg_execution_time_ms,
      plan_count: baseline.accepted_plans.length,
    };
  }

  /**
   * Format explain plan as tree
   *
   * Convert an explain plan to a human-readable tree format.
   *
   * @param plan - Explain plan
   * @param indent - Indentation level (used internally)
   * @returns Formatted plan tree
   */
  formatPlanTree(plan: ExplainPlan, indent: number = 0): string {
    const prefix = '  '.repeat(indent);
    let output = `${prefix}${plan.operator} (cost=${plan.cost.toFixed(2)}, rows=${plan.rows})\n`;

    if (Object.keys(plan.details).length > 0) {
      output += `${prefix}  Details: ${JSON.stringify(plan.details)}\n`;
    }

    for (const child of plan.children) {
      output += this.formatPlanTree(child, indent + 1);
    }

    return output;
  }
}

// ============================================================================
// Default Export
// ============================================================================

export default QueryOptimizerClient;
