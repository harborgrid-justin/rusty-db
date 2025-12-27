/**
 * Machine Learning & Analytics API Client for RustyDB
 *
 * Provides comprehensive TypeScript client for:
 * - ML model management (CRUD, training, prediction, evaluation)
 * - OLAP operations (cubes, queries, dimensions)
 * - Analytics (query statistics, workload analysis)
 * - In-memory column store operations
 * - Data quality analysis
 * - Materialized views
 *
 * @module ml-analytics
 */

import axios, { AxiosInstance } from 'axios';

// ============================================================================
// Machine Learning Types
// ============================================================================

/**
 * Supported ML model types
 */
export type MLModelType =
  | 'linear_regression'
  | 'logistic_regression'
  | 'kmeans'
  | 'decision_tree'
  | 'random_forest';

/**
 * Model training status
 */
export type ModelStatus = 'created' | 'training' | 'trained' | 'failed' | 'deployed';

/**
 * Hyperparameters for model training
 */
export interface Hyperparameters {
  [key: string]: number | string | boolean;
}

/**
 * Request to create a new ML model
 */
export interface CreateModelRequest {
  name: string;
  model_type: MLModelType;
  hyperparameters?: Hyperparameters;
  description?: string;
}

/**
 * ML model information
 */
export interface MLModel {
  model_id: string;
  name: string;
  model_type: string;
  status: ModelStatus;
  created_at: number;
  version: number;
}

/**
 * Request to train a model
 */
export interface TrainModelRequest {
  data_query?: string;
  features?: number[][];
  target?: number[];
  feature_names?: string[];
  validation_split?: number;
  epochs?: number;
}

/**
 * Training result
 */
export interface TrainingJob {
  model_id: string;
  status: string;
  metrics: Record<string, number>;
  training_time_ms: number;
  epochs_completed: number;
}

/**
 * Prediction request
 */
export interface PredictRequest {
  features: number[][];
  feature_names?: string[];
}

/**
 * Prediction result
 */
export interface Prediction {
  predictions: number[];
  confidence_scores?: number[];
  prediction_count: number;
}

/**
 * Model summary in list view
 */
export interface ModelSummary {
  model_id: string;
  name: string;
  model_type: string;
  status: string;
  accuracy?: number;
  created_at: number;
}

/**
 * Model list response
 */
export interface ModelListResponse {
  models: ModelSummary[];
  total_count: number;
}

/**
 * Feature importance
 */
export interface FeatureImportance {
  feature_name: string;
  importance: number;
}

/**
 * Model metrics
 */
export interface ModelMetrics {
  model_id: string;
  metrics: Record<string, number>;
  feature_importance?: FeatureImportance[];
}

/**
 * Model evaluation request
 */
export interface ModelEvaluationRequest {
  test_features: number[][];
  test_target: number[];
  metrics?: string[];
}

/**
 * Model evaluation response
 */
export interface ModelEvaluationResponse {
  model_id: string;
  metrics: Record<string, number>;
  confusion_matrix?: number[][];
}

// ============================================================================
// Analytics - OLAP Types
// ============================================================================

/**
 * Aggregation function for OLAP measures
 */
export type AggregateFunction = 'SUM' | 'AVG' | 'COUNT' | 'MIN' | 'MAX';

/**
 * Measure specification for OLAP cube
 */
export interface MeasureSpec {
  column: string;
  aggregation: AggregateFunction;
}

/**
 * Request to create OLAP cube
 */
export interface CreateCubeRequest {
  name: string;
  dimensions: string[];
  measures: MeasureSpec[];
  source: string;
}

/**
 * OLAP cube information
 */
export interface OLAPCube {
  id: string;
  name: string;
  dimensions: string[];
  measures: string[];
  created_at: number;
  size_bytes: number;
}

/**
 * OLAP cube list response
 */
export interface CubeListResponse {
  cubes: OLAPCube[];
  total_count: number;
}

/**
 * OLAP operation type
 */
export type OLAPOperation = 'drill-down' | 'roll-up' | 'slice' | 'dice';

/**
 * OLAP cube query request
 */
export interface CubeQueryRequest {
  filters: Record<string, string>;
  operation?: OLAPOperation;
  target_dimension?: string;
}

/**
 * OLAP cube query response
 */
export interface CubeQueryResponse {
  results: Record<string, unknown>[];
  row_count: number;
  execution_time_ms: number;
}

// ============================================================================
// Analytics - Query Statistics Types
// ============================================================================

/**
 * Query statistics filter
 */
export interface QueryStatsFilter {
  time_range_hours?: number;
  min_execution_time_ms?: number;
  table_name?: string;
  limit?: number;
}

/**
 * Query statistics entry
 */
export interface QueryStatEntry {
  query_id: number;
  normalized_sql: string;
  execution_count: number;
  avg_execution_time_ms: number;
  min_execution_time_ms: number;
  max_execution_time_ms: number;
  total_rows_examined: number;
  total_rows_returned: number;
  last_executed: number;
}

/**
 * Query statistics response
 */
export interface QueryStatisticsResponse {
  statistics: QueryStatEntry[];
  total_queries: number;
  avg_execution_time_ms: number;
  slow_query_count: number;
}

/**
 * Recommendation priority
 */
export type RecommendationPriority = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

/**
 * Recommendation type
 */
export type RecommendationType = 'INDEX' | 'PARTITION' | 'QUERY_REWRITE' | 'STATISTICS' | 'CONFIGURATION';

/**
 * Optimization recommendation
 */
export interface RecommendationEntry {
  recommendation_type: RecommendationType;
  priority: RecommendationPriority;
  description: string;
  affected_tables: string[];
  affected_columns: string[];
  estimated_improvement: number;
}

/**
 * Workload analysis response
 */
export interface WorkloadAnalysisResponse {
  analysis_timestamp: number;
  total_queries: number;
  unique_patterns: number;
  recommendations: RecommendationEntry[];
  top_queries: QueryStatEntry[];
  table_access_patterns: Record<string, number>;
}

// ============================================================================
// Analytics - Data Quality Types
// ============================================================================

/**
 * Table profiling request
 */
export interface ProfileTableRequest {
  sample_percent?: number;
  include_patterns?: boolean;
  suggest_indexes?: boolean;
}

/**
 * Column profile information
 */
export interface ColumnProfileEntry {
  column_name: string;
  inferred_type: string;
  null_count: number;
  null_percentage: number;
  distinct_count: number;
  cardinality: number;
  min_value?: string;
  max_value?: string;
  avg_length?: number;
}

/**
 * Index suggestion
 */
export interface IndexSuggestionEntry {
  index_type: string;
  columns: string[];
  reason: string;
  estimated_benefit: string;
}

/**
 * Table profile response
 */
export interface ProfileTableResponse {
  table_name: string;
  row_count: number;
  column_profiles: ColumnProfileEntry[];
  index_suggestions: IndexSuggestionEntry[];
  profiled_at: number;
}

/**
 * Data quality metrics
 */
export interface QualityMetrics {
  table_name: string;
  overall_score: number;
  completeness: number;
  uniqueness: number;
  validity: number;
  consistency: number;
  accuracy: number;
  timeliness: number;
  row_count: number;
  issue_count: number;
  analyzed_at: number;
}

/**
 * Quality issue severity
 */
export type IssueSeverity = 'INFO' | 'WARNING' | 'CRITICAL';

/**
 * Quality issue type
 */
export type IssueType = 'NULL_VALUE' | 'DUPLICATE' | 'INVALID_FORMAT' | 'OUTLIER' | 'INCONSISTENT';

/**
 * Data quality issue
 */
export interface QualityIssueEntry {
  issue_type: IssueType;
  severity: IssueSeverity;
  column_name?: string;
  row_number?: number;
  description: string;
  suggested_fix?: string;
}

/**
 * Quality issues response
 */
export interface QualityIssuesResponse {
  table_name: string;
  issues: QualityIssueEntry[];
  total_count: number;
  critical_count: number;
  warning_count: number;
}

// ============================================================================
// Analytics - Materialized Views Types
// ============================================================================

/**
 * Refresh schedule specification
 */
export interface RefreshScheduleSpec {
  interval_secs: number;
  auto_refresh: boolean;
}

/**
 * Create materialized view request
 */
export interface CreateMaterializedViewRequest {
  name: string;
  query: string;
  refresh_schedule?: RefreshScheduleSpec;
  indexes?: string[];
}

/**
 * Materialized view information
 */
export interface MaterializedView {
  id: string;
  name: string;
  query: string;
  row_count: number;
  last_refreshed: number;
  next_refresh?: number;
  size_bytes: number;
  indexes: string[];
}

/**
 * Materialized view list response
 */
export interface MaterializedViewListResponse {
  views: MaterializedView[];
  total_count: number;
}

/**
 * Refresh materialized view response
 */
export interface RefreshMaterializedViewResponse {
  view_id: string;
  view_name: string;
  rows_refreshed: number;
  refresh_time_ms: number;
  refreshed_at: number;
}

// ============================================================================
// In-Memory Column Store Types
// ============================================================================

/**
 * In-memory priority level
 */
export type InMemoryPriority = 'low' | 'medium' | 'high';

/**
 * Population strategy
 */
export type PopulationStrategy = 'full' | 'incremental';

/**
 * Enable in-memory request
 */
export interface EnableInMemoryRequest {
  table: string;
  columns?: string[];
  priority?: InMemoryPriority;
  compression?: boolean;
}

/**
 * Enable in-memory response
 */
export interface EnableInMemoryResponse {
  table: string;
  status: string;
  population_started: boolean;
  estimated_size_bytes: number;
}

/**
 * In-memory table information
 */
export interface InMemoryTableInfo {
  table_name: string;
  memory_bytes: number;
  row_count: number;
  compression_ratio: number;
  population_status: string;
  last_accessed: number;
}

/**
 * In-memory status response
 */
export interface InMemoryStatusResponse {
  enabled: boolean;
  total_memory_bytes: number;
  used_memory_bytes: number;
  memory_utilization_percent: number;
  tables: InMemoryTableInfo[];
}

/**
 * Populate table request
 */
export interface PopulateRequest {
  table: string;
  force?: boolean;
  strategy?: PopulationStrategy;
}

/**
 * Populate table response
 */
export interface PopulateResponse {
  table: string;
  status: string;
  rows_populated: number;
  duration_ms: number;
}

/**
 * Evict tables request
 */
export interface EvictRequest {
  table?: string;
  threshold_percent?: number;
}

/**
 * Evict tables response
 */
export interface EvictResponse {
  tables_evicted: string[];
  memory_freed_bytes: number;
}

/**
 * In-memory statistics
 */
export interface InMemoryStats {
  total_stores: number;
  total_memory_bytes: number;
  max_memory_bytes: number;
  memory_pressure: number;
  cache_hits: number;
  cache_misses: number;
  cache_hit_ratio: number;
  population_queue_size: number;
}

/**
 * In-memory configuration
 */
export interface InMemoryConfig {
  max_memory_bytes: number;
  auto_populate: boolean;
  enable_compression: boolean;
  vector_width: number;
  cache_line_size: number;
}

// ============================================================================
// API Error Type
// ============================================================================

/**
 * API error response
 */
export interface ApiError {
  error: string;
  message: string;
  details?: unknown;
}

// ============================================================================
// ML & Analytics Client Configuration
// ============================================================================

/**
 * Client configuration options
 */
export interface MLAnalyticsClientConfig {
  baseUrl: string;
  timeout?: number;
  headers?: Record<string, string>;
  apiVersion?: string;
}

// ============================================================================
// ML & Analytics Client
// ============================================================================

/**
 * Comprehensive ML & Analytics API Client for RustyDB
 *
 * Provides full coverage for:
 * - Machine Learning: Model CRUD, training, prediction, evaluation
 * - OLAP: Cube management and queries
 * - Analytics: Query stats, workload analysis, recommendations
 * - Data Quality: Profiling, metrics, issue detection
 * - Materialized Views: Create, refresh, manage
 * - In-Memory: Column store management and operations
 */
export class MLAnalyticsClient {
  private client: AxiosInstance;
  private readonly apiVersion: string;

  /**
   * Create a new ML & Analytics client
   *
   * @param config - Client configuration
   *
   * @example
   * ```typescript
   * const client = new MLAnalyticsClient({
   *   baseUrl: 'http://localhost:5432',
   *   timeout: 30000,
   *   apiVersion: 'v1'
   * });
   * ```
   */
  constructor(config: MLAnalyticsClientConfig) {
    this.apiVersion = config.apiVersion || 'v1';

    this.client = axios.create({
      baseURL: config.baseUrl,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        ...config.headers,
      },
    });
  }

  // ========================================================================
  // Machine Learning - Model Management
  // ========================================================================

  /**
   * Create a new machine learning model
   *
   * @param request - Model creation parameters
   * @returns Created model information
   *
   * @example
   * ```typescript
   * const model = await client.createModel({
   *   name: 'customer_churn_model',
   *   model_type: 'logistic_regression',
   *   hyperparameters: {
   *     learning_rate: 0.01,
   *     max_iterations: 1000
   *   },
   *   description: 'Predict customer churn probability'
   * });
   * ```
   */
  async createModel(request: CreateModelRequest): Promise<MLModel> {
    const response = await this.client.post<MLModel>(
      `/api/${this.apiVersion}/ml/models`,
      request
    );
    return response.data;
  }

  /**
   * List all ML models
   *
   * @returns List of all models
   *
   * @example
   * ```typescript
   * const { models, total_count } = await client.listModels();
   * console.log(`Found ${total_count} models`);
   * ```
   */
  async listModels(): Promise<ModelListResponse> {
    const response = await this.client.get<ModelListResponse>(
      `/api/${this.apiVersion}/ml/models`
    );
    return response.data;
  }

  /**
   * Get model details by ID
   *
   * @param modelId - Model identifier
   * @returns Model information
   *
   * @example
   * ```typescript
   * const model = await client.getModel('customer_churn_model');
   * console.log(`Model status: ${model.status}`);
   * ```
   */
  async getModel(modelId: string): Promise<MLModel> {
    const response = await this.client.get<MLModel>(
      `/api/${this.apiVersion}/ml/models/${modelId}`
    );
    return response.data;
  }

  /**
   * Delete a model
   *
   * @param modelId - Model identifier
   *
   * @example
   * ```typescript
   * await client.deleteModel('old_model');
   * ```
   */
  async deleteModel(modelId: string): Promise<void> {
    await this.client.delete(`/api/${this.apiVersion}/ml/models/${modelId}`);
  }

  // ========================================================================
  // Machine Learning - Training & Prediction
  // ========================================================================

  /**
   * Train a machine learning model
   *
   * @param modelId - Model identifier
   * @param request - Training parameters and data
   * @returns Training results with metrics
   *
   * @example
   * ```typescript
   * const result = await client.trainModel('customer_churn_model', {
   *   features: [[1.0, 2.0], [3.0, 4.0]],
   *   target: [0, 1],
   *   feature_names: ['age', 'spend'],
   *   validation_split: 0.2,
   *   epochs: 100
   * });
   * console.log(`Training completed in ${result.training_time_ms}ms`);
   * console.log(`Accuracy: ${result.metrics.accuracy}`);
   * ```
   */
  async trainModel(modelId: string, request: TrainModelRequest): Promise<TrainingJob> {
    const response = await this.client.post<TrainingJob>(
      `/api/${this.apiVersion}/ml/models/${modelId}/train`,
      request
    );
    return response.data;
  }

  /**
   * Make predictions with a trained model
   *
   * @param modelId - Model identifier
   * @param request - Features for prediction
   * @returns Predictions with optional confidence scores
   *
   * @example
   * ```typescript
   * const predictions = await client.predict('customer_churn_model', {
   *   features: [[5.0, 100.0], [6.0, 150.0]],
   *   feature_names: ['age', 'spend']
   * });
   * predictions.predictions.forEach((pred, i) => {
   *   console.log(`Customer ${i}: ${pred > 0.5 ? 'likely to churn' : 'likely to stay'}`);
   * });
   * ```
   */
  async predict(modelId: string, request: PredictRequest): Promise<Prediction> {
    const response = await this.client.post<Prediction>(
      `/api/${this.apiVersion}/ml/models/${modelId}/predict`,
      request
    );
    return response.data;
  }

  /**
   * Get model metrics and feature importance
   *
   * @param modelId - Model identifier
   * @returns Model metrics and feature importance
   *
   * @example
   * ```typescript
   * const metrics = await client.getModelMetrics('customer_churn_model');
   * console.log(`Model accuracy: ${metrics.metrics.accuracy}`);
   * if (metrics.feature_importance) {
   *   metrics.feature_importance.forEach(fi => {
   *     console.log(`${fi.feature_name}: ${fi.importance}`);
   *   });
   * }
   * ```
   */
  async getModelMetrics(modelId: string): Promise<ModelMetrics> {
    const response = await this.client.get<ModelMetrics>(
      `/api/${this.apiVersion}/ml/models/${modelId}/metrics`
    );
    return response.data;
  }

  /**
   * Evaluate a model on test data
   *
   * @param modelId - Model identifier
   * @param request - Test data and metrics to calculate
   * @returns Evaluation metrics
   *
   * @example
   * ```typescript
   * const evaluation = await client.evaluateModel('customer_churn_model', {
   *   test_features: [[7.0, 200.0], [8.0, 250.0]],
   *   test_target: [1, 0],
   *   metrics: ['accuracy', 'precision', 'recall', 'f1']
   * });
   * console.log(`Test accuracy: ${evaluation.metrics.accuracy}`);
   * ```
   */
  async evaluateModel(modelId: string, request: ModelEvaluationRequest): Promise<ModelEvaluationResponse> {
    const response = await this.client.post<ModelEvaluationResponse>(
      `/api/${this.apiVersion}/ml/models/${modelId}/evaluate`,
      request
    );
    return response.data;
  }

  /**
   * Export a trained model
   *
   * @param modelId - Model identifier
   * @returns Model export data
   *
   * @example
   * ```typescript
   * const exportData = await client.exportModel('customer_churn_model');
   * // Save to file or transfer to another system
   * ```
   */
  async exportModel(modelId: string): Promise<unknown> {
    const response = await this.client.get(
      `/api/${this.apiVersion}/ml/models/${modelId}/export`
    );
    return response.data;
  }

  // ========================================================================
  // Analytics - OLAP Operations
  // ========================================================================

  /**
   * Create a new OLAP cube
   *
   * @param request - Cube definition
   * @returns Created cube information
   *
   * @example
   * ```typescript
   * const cube = await client.createOLAPCube({
   *   name: 'sales_cube',
   *   dimensions: ['region', 'product', 'time'],
   *   measures: [
   *     { column: 'revenue', aggregation: 'SUM' },
   *     { column: 'quantity', aggregation: 'SUM' }
   *   ],
   *   source: 'sales_table'
   * });
   * ```
   */
  async createOLAPCube(request: CreateCubeRequest): Promise<OLAPCube> {
    const response = await this.client.post<OLAPCube>(
      `/api/${this.apiVersion}/analytics/olap/cubes`,
      request
    );
    return response.data;
  }

  /**
   * List all OLAP cubes
   *
   * @returns List of OLAP cubes
   *
   * @example
   * ```typescript
   * const { cubes, total_count } = await client.listOLAPCubes();
   * ```
   */
  async listOLAPCubes(): Promise<CubeListResponse> {
    const response = await this.client.get<CubeListResponse>(
      `/api/${this.apiVersion}/analytics/olap/cubes`
    );
    return response.data;
  }

  /**
   * Query an OLAP cube
   *
   * @param cubeId - Cube identifier
   * @param request - Query parameters
   * @returns Query results
   *
   * @example
   * ```typescript
   * const results = await client.queryOLAPCube('sales_cube', {
   *   filters: { region: 'North', time: '2024-Q1' },
   *   operation: 'drill-down',
   *   target_dimension: 'product'
   * });
   * ```
   */
  async queryOLAPCube(cubeId: string, request: CubeQueryRequest): Promise<CubeQueryResponse> {
    const response = await this.client.post<CubeQueryResponse>(
      `/api/${this.apiVersion}/analytics/olap/cubes/${cubeId}/query`,
      request
    );
    return response.data;
  }

  /**
   * Delete an OLAP cube
   *
   * @param cubeId - Cube identifier
   *
   * @example
   * ```typescript
   * await client.deleteOLAPCube('old_cube');
   * ```
   */
  async deleteOLAPCube(cubeId: string): Promise<void> {
    await this.client.delete(
      `/api/${this.apiVersion}/analytics/olap/cubes/${cubeId}`
    );
  }

  // ========================================================================
  // Analytics - Query Statistics & Workload Analysis
  // ========================================================================

  /**
   * Get query statistics and performance metrics
   *
   * @param filter - Optional filters
   * @returns Query statistics
   *
   * @example
   * ```typescript
   * const stats = await client.getQueryStatistics({
   *   time_range_hours: 24,
   *   min_execution_time_ms: 1000,
   *   limit: 50
   * });
   * console.log(`Total queries: ${stats.total_queries}`);
   * console.log(`Slow queries: ${stats.slow_query_count}`);
   * ```
   */
  async getQueryStatistics(filter?: QueryStatsFilter): Promise<QueryStatisticsResponse> {
    const response = await this.client.get<QueryStatisticsResponse>(
      `/api/${this.apiVersion}/analytics/query-stats`,
      { params: filter }
    );
    return response.data;
  }

  /**
   * Analyze workload patterns and get recommendations
   *
   * @returns Workload analysis with recommendations
   *
   * @example
   * ```typescript
   * const analysis = await client.analyzeWorkload();
   * console.log(`Analyzed ${analysis.total_queries} queries`);
   * analysis.recommendations.forEach(rec => {
   *   console.log(`${rec.priority}: ${rec.description}`);
   * });
   * ```
   */
  async analyzeWorkload(): Promise<WorkloadAnalysisResponse> {
    const response = await this.client.get<WorkloadAnalysisResponse>(
      `/api/${this.apiVersion}/analytics/workload`
    );
    return response.data;
  }

  /**
   * Get optimization recommendations
   *
   * @returns List of recommendations
   *
   * @example
   * ```typescript
   * const recommendations = await client.getRecommendations();
   * const highPriority = recommendations.filter(r => r.priority === 'HIGH');
   * ```
   */
  async getRecommendations(): Promise<RecommendationEntry[]> {
    const response = await this.client.get<RecommendationEntry[]>(
      `/api/${this.apiVersion}/analytics/recommendations`
    );
    return response.data;
  }

  // ========================================================================
  // Analytics - Data Quality
  // ========================================================================

  /**
   * Profile a table to analyze data characteristics
   *
   * @param tableName - Table name
   * @param request - Profiling options
   * @returns Table profiling results
   *
   * @example
   * ```typescript
   * const profile = await client.profileTable('customers', {
   *   sample_percent: 10,
   *   include_patterns: true,
   *   suggest_indexes: true
   * });
   * console.log(`Analyzed ${profile.row_count} rows`);
   * profile.index_suggestions.forEach(suggestion => {
   *   console.log(`Suggested ${suggestion.index_type} on ${suggestion.columns.join(', ')}`);
   * });
   * ```
   */
  async profileTable(tableName: string, request?: ProfileTableRequest): Promise<ProfileTableResponse> {
    const response = await this.client.post<ProfileTableResponse>(
      `/api/${this.apiVersion}/analytics/profile/${tableName}`,
      request || {}
    );
    return response.data;
  }

  /**
   * Get data quality metrics for a table
   *
   * @param tableName - Table name
   * @returns Quality metrics
   *
   * @example
   * ```typescript
   * const quality = await client.getQualityMetrics('customers');
   * console.log(`Overall quality score: ${quality.overall_score}`);
   * console.log(`Completeness: ${quality.completeness * 100}%`);
   * ```
   */
  async getQualityMetrics(tableName: string): Promise<QualityMetrics> {
    const response = await this.client.get<QualityMetrics>(
      `/api/${this.apiVersion}/analytics/quality/${tableName}`
    );
    return response.data;
  }

  /**
   * Get data quality issues for a table
   *
   * @param tableName - Table name
   * @returns Quality issues
   *
   * @example
   * ```typescript
   * const issues = await client.getQualityIssues('customers');
   * console.log(`Found ${issues.total_count} issues`);
   * const critical = issues.issues.filter(i => i.severity === 'CRITICAL');
   * ```
   */
  async getQualityIssues(tableName: string): Promise<QualityIssuesResponse> {
    const response = await this.client.get<QualityIssuesResponse>(
      `/api/${this.apiVersion}/analytics/quality/${tableName}/issues`
    );
    return response.data;
  }

  // ========================================================================
  // Analytics - Materialized Views
  // ========================================================================

  /**
   * Create a new materialized view
   *
   * @param request - View definition
   * @returns Created view information
   *
   * @example
   * ```typescript
   * const view = await client.createMaterializedView({
   *   name: 'monthly_sales_summary',
   *   query: 'SELECT region, SUM(revenue) FROM sales GROUP BY region',
   *   refresh_schedule: {
   *     interval_secs: 3600,
   *     auto_refresh: true
   *   },
   *   indexes: ['region']
   * });
   * ```
   */
  async createMaterializedView(request: CreateMaterializedViewRequest): Promise<MaterializedView> {
    const response = await this.client.post<MaterializedView>(
      `/api/${this.apiVersion}/analytics/materialized-views`,
      request
    );
    return response.data;
  }

  /**
   * List all materialized views
   *
   * @returns List of materialized views
   *
   * @example
   * ```typescript
   * const { views, total_count } = await client.listMaterializedViews();
   * ```
   */
  async listMaterializedViews(): Promise<MaterializedViewListResponse> {
    const response = await this.client.get<MaterializedViewListResponse>(
      `/api/${this.apiVersion}/analytics/materialized-views`
    );
    return response.data;
  }

  /**
   * Refresh a materialized view
   *
   * @param viewId - View identifier
   * @returns Refresh results
   *
   * @example
   * ```typescript
   * const result = await client.refreshMaterializedView('monthly_sales_summary');
   * console.log(`Refreshed ${result.rows_refreshed} rows in ${result.refresh_time_ms}ms`);
   * ```
   */
  async refreshMaterializedView(viewId: string): Promise<RefreshMaterializedViewResponse> {
    const response = await this.client.post<RefreshMaterializedViewResponse>(
      `/api/${this.apiVersion}/analytics/materialized-views/${viewId}/refresh`
    );
    return response.data;
  }

  // ========================================================================
  // In-Memory Column Store
  // ========================================================================

  /**
   * Enable in-memory storage for a table
   *
   * @param request - In-memory configuration
   * @returns Enable response
   *
   * @example
   * ```typescript
   * const result = await client.enableInMemory({
   *   table: 'hot_data',
   *   columns: ['id', 'value', 'timestamp'],
   *   priority: 'high',
   *   compression: true
   * });
   * console.log(`Table ${result.table} enabled, population started: ${result.population_started}`);
   * ```
   */
  async enableInMemory(request: EnableInMemoryRequest): Promise<EnableInMemoryResponse> {
    const response = await this.client.post<EnableInMemoryResponse>(
      `/api/${this.apiVersion}/inmemory/enable`,
      request
    );
    return response.data;
  }

  /**
   * Disable in-memory storage for a table
   *
   * @param tableName - Table name
   *
   * @example
   * ```typescript
   * await client.disableInMemory('old_hot_data');
   * ```
   */
  async disableInMemory(tableName: string): Promise<void> {
    await this.client.post(
      `/api/${this.apiVersion}/inmemory/disable`,
      null,
      { params: { table: tableName } }
    );
  }

  /**
   * Get in-memory status
   *
   * @returns In-memory status
   *
   * @example
   * ```typescript
   * const status = await client.getInMemoryStatus();
   * console.log(`Memory utilization: ${status.memory_utilization_percent}%`);
   * console.log(`Tables in memory: ${status.tables.length}`);
   * ```
   */
  async getInMemoryStatus(): Promise<InMemoryStatusResponse> {
    const response = await this.client.get<InMemoryStatusResponse>(
      `/api/${this.apiVersion}/inmemory/status`
    );
    return response.data;
  }

  /**
   * Get in-memory statistics
   *
   * @returns In-memory statistics
   *
   * @example
   * ```typescript
   * const stats = await client.getInMemoryStats();
   * console.log(`Cache hit ratio: ${stats.cache_hit_ratio * 100}%`);
   * console.log(`Memory pressure: ${stats.memory_pressure}`);
   * ```
   */
  async getInMemoryStats(): Promise<InMemoryStats> {
    const response = await this.client.get<InMemoryStats>(
      `/api/${this.apiVersion}/inmemory/stats`
    );
    return response.data;
  }

  /**
   * Populate a table into memory
   *
   * @param request - Population parameters
   * @returns Population results
   *
   * @example
   * ```typescript
   * const result = await client.populateTable({
   *   table: 'products',
   *   force: true,
   *   strategy: 'full'
   * });
   * console.log(`Populated ${result.rows_populated} rows in ${result.duration_ms}ms`);
   * ```
   */
  async populateTable(request: PopulateRequest): Promise<PopulateResponse> {
    const response = await this.client.post<PopulateResponse>(
      `/api/${this.apiVersion}/inmemory/populate`,
      request
    );
    return response.data;
  }

  /**
   * Evict tables from memory
   *
   * @param request - Eviction parameters
   * @returns Eviction results
   *
   * @example
   * ```typescript
   * // Evict specific table
   * const result1 = await client.evictTables({ table: 'old_data' });
   *
   * // Evict based on memory pressure
   * const result2 = await client.evictTables({ threshold_percent: 80 });
   * console.log(`Freed ${result2.memory_freed_bytes} bytes`);
   * ```
   */
  async evictTables(request: EvictRequest): Promise<EvictResponse> {
    const response = await this.client.post<EvictResponse>(
      `/api/${this.apiVersion}/inmemory/evict`,
      request
    );
    return response.data;
  }

  /**
   * Get table population status
   *
   * @param tableName - Table name
   * @returns Table status
   *
   * @example
   * ```typescript
   * const status = await client.getTableStatus('products');
   * console.log(`Compression ratio: ${status.compression_ratio}`);
   * console.log(`Memory usage: ${status.memory_bytes} bytes`);
   * ```
   */
  async getTableStatus(tableName: string): Promise<InMemoryTableInfo> {
    const response = await this.client.get<InMemoryTableInfo>(
      `/api/${this.apiVersion}/inmemory/tables/${tableName}/status`
    );
    return response.data;
  }

  /**
   * Force memory compaction
   *
   * @example
   * ```typescript
   * await client.compactMemory();
   * ```
   */
  async compactMemory(): Promise<void> {
    await this.client.post(`/api/${this.apiVersion}/inmemory/compact`);
  }

  /**
   * Update in-memory configuration
   *
   * @param config - Configuration parameters
   *
   * @example
   * ```typescript
   * await client.updateInMemoryConfig({
   *   max_memory_bytes: 8 * 1024 * 1024 * 1024,
   *   auto_populate: true,
   *   enable_compression: true
   * });
   * ```
   */
  async updateInMemoryConfig(config: Partial<InMemoryConfig>): Promise<void> {
    await this.client.put(
      `/api/${this.apiVersion}/inmemory/config`,
      config
    );
  }

  /**
   * Get in-memory configuration
   *
   * @returns Current configuration
   *
   * @example
   * ```typescript
   * const config = await client.getInMemoryConfig();
   * console.log(`Max memory: ${config.max_memory_bytes} bytes`);
   * ```
   */
  async getInMemoryConfig(): Promise<InMemoryConfig> {
    const response = await this.client.get<InMemoryConfig>(
      `/api/${this.apiVersion}/inmemory/config`
    );
    return response.data;
  }
}

// ============================================================================
// Exports
// ============================================================================

export default MLAnalyticsClient;
