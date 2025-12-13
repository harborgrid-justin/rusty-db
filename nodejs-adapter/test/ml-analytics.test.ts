/**
 * Comprehensive Test Suite for ML & Analytics API Client
 *
 * Tests all endpoints:
 * - ML Model Management (CRUD, training, prediction, evaluation)
 * - OLAP Operations (cubes, queries)
 * - Query Statistics & Workload Analysis
 * - Data Quality Analysis
 * - Materialized Views
 * - In-Memory Column Store
 *
 * @module ml-analytics.test
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from '@jest/globals';
import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import {
  MLAnalyticsClient,
  CreateModelRequest,
  TrainModelRequest,
  PredictRequest,
  ModelEvaluationRequest,
  CreateCubeRequest,
  CubeQueryRequest,
  QueryStatsFilter,
  ProfileTableRequest,
  CreateMaterializedViewRequest,
  EnableInMemoryRequest,
  PopulateRequest,
  EvictRequest,
  MLModel,
  TrainingJob,
  Prediction,
  ModelMetrics,
  ModelEvaluationResponse,
  OLAPCube,
  CubeQueryResponse,
  QueryStatisticsResponse,
  WorkloadAnalysisResponse,
  RecommendationEntry,
  ProfileTableResponse,
  QualityMetrics,
  QualityIssuesResponse,
  MaterializedView,
  RefreshMaterializedViewResponse,
  EnableInMemoryResponse,
  InMemoryStatusResponse,
  InMemoryStats,
  PopulateResponse,
  EvictResponse,
  InMemoryTableInfo,
  InMemoryConfig,
} from '../src/api/ml-analytics';

describe('MLAnalyticsClient', () => {
  let client: MLAnalyticsClient;
  let mock: MockAdapter;

  beforeAll(() => {
    client = new MLAnalyticsClient({
      baseUrl: 'http://localhost:5432',
      timeout: 30000,
      apiVersion: 'v1',
    });
    mock = new MockAdapter(axios);
  });

  afterAll(() => {
    mock.restore();
  });

  beforeEach(() => {
    mock.reset();
  });

  // =========================================================================
  // Machine Learning - Model Management Tests
  // =========================================================================

  describe('Machine Learning - Model Management', () => {
    describe('createModel', () => {
      it('should create a linear regression model', async () => {
        const request: CreateModelRequest = {
          name: 'sales_predictor',
          model_type: 'linear_regression',
          hyperparameters: {
            learning_rate: 0.01,
            max_iterations: 1000,
          },
          description: 'Predict monthly sales',
        };

        const mockResponse: MLModel = {
          model_id: 'model-123',
          name: 'sales_predictor',
          model_type: 'linear_regression',
          status: 'created',
          created_at: Date.now(),
          version: 1,
        };

        mock.onPost('/api/v1/ml/models').reply(201, mockResponse);

        const result = await client.createModel(request);

        expect(result.model_id).toBe('model-123');
        expect(result.name).toBe('sales_predictor');
        expect(result.status).toBe('created');
      });

      it('should create a logistic regression model', async () => {
        const request: CreateModelRequest = {
          name: 'churn_classifier',
          model_type: 'logistic_regression',
          hyperparameters: {
            regularization: 0.1,
          },
        };

        const mockResponse: MLModel = {
          model_id: 'model-456',
          name: 'churn_classifier',
          model_type: 'logistic_regression',
          status: 'created',
          created_at: Date.now(),
          version: 1,
        };

        mock.onPost('/api/v1/ml/models').reply(201, mockResponse);

        const result = await client.createModel(request);

        expect(result.model_type).toBe('logistic_regression');
      });

      it('should create a decision tree model', async () => {
        const request: CreateModelRequest = {
          name: 'product_recommender',
          model_type: 'decision_tree',
          hyperparameters: {
            max_depth: 10,
            min_samples_split: 5,
          },
        };

        const mockResponse: MLModel = {
          model_id: 'model-789',
          name: 'product_recommender',
          model_type: 'decision_tree',
          status: 'created',
          created_at: Date.now(),
          version: 1,
        };

        mock.onPost('/api/v1/ml/models').reply(201, mockResponse);

        const result = await client.createModel(request);

        expect(result.model_type).toBe('decision_tree');
      });
    });

    describe('listModels', () => {
      it('should list all models', async () => {
        const mockResponse = {
          models: [
            {
              model_id: 'model-1',
              name: 'model_1',
              model_type: 'linear_regression',
              status: 'trained',
              accuracy: 0.95,
              created_at: Date.now(),
            },
            {
              model_id: 'model-2',
              name: 'model_2',
              model_type: 'logistic_regression',
              status: 'created',
              created_at: Date.now(),
            },
          ],
          total_count: 2,
        };

        mock.onGet('/api/v1/ml/models').reply(200, mockResponse);

        const result = await client.listModels();

        expect(result.total_count).toBe(2);
        expect(result.models).toHaveLength(2);
        expect(result.models[0].model_id).toBe('model-1');
      });

      it('should handle empty model list', async () => {
        const mockResponse = {
          models: [],
          total_count: 0,
        };

        mock.onGet('/api/v1/ml/models').reply(200, mockResponse);

        const result = await client.listModels();

        expect(result.total_count).toBe(0);
        expect(result.models).toHaveLength(0);
      });
    });

    describe('getModel', () => {
      it('should get model by ID', async () => {
        const mockResponse: MLModel = {
          model_id: 'sales_predictor',
          name: 'sales_predictor',
          model_type: 'LinearRegression',
          status: 'trained',
          created_at: Date.now(),
          version: 1,
        };

        mock.onGet('/api/v1/ml/models/sales_predictor').reply(200, mockResponse);

        const result = await client.getModel('sales_predictor');

        expect(result.model_id).toBe('sales_predictor');
        expect(result.status).toBe('trained');
      });

      it('should handle model not found', async () => {
        mock.onGet('/api/v1/ml/models/nonexistent').reply(404, {
          error: 'NOT_FOUND',
          message: 'Model not found',
        });

        await expect(client.getModel('nonexistent')).rejects.toThrow();
      });
    });

    describe('deleteModel', () => {
      it('should delete a model', async () => {
        mock.onDelete('/api/v1/ml/models/old_model').reply(204);

        await expect(client.deleteModel('old_model')).resolves.not.toThrow();
      });
    });
  });

  // =========================================================================
  // Machine Learning - Training & Prediction Tests
  // =========================================================================

  describe('Machine Learning - Training & Prediction', () => {
    describe('trainModel', () => {
      it('should train a model with feature data', async () => {
        const request: TrainModelRequest = {
          features: [
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
          ],
          target: [10.0, 20.0, 30.0],
          feature_names: ['feature_1', 'feature_2', 'feature_3'],
          validation_split: 0.2,
          epochs: 100,
        };

        const mockResponse: TrainingJob = {
          model_id: 'sales_predictor',
          status: 'trained',
          metrics: {
            accuracy: 0.95,
            mse: 0.05,
            rmse: 0.22,
            r2: 0.92,
          },
          training_time_ms: 1500,
          epochs_completed: 100,
        };

        mock.onPost('/api/v1/ml/models/sales_predictor/train').reply(200, mockResponse);

        const result = await client.trainModel('sales_predictor', request);

        expect(result.status).toBe('trained');
        expect(result.metrics.accuracy).toBe(0.95);
        expect(result.training_time_ms).toBe(1500);
        expect(result.epochs_completed).toBe(100);
      });

      it('should train with SQL query', async () => {
        const request: TrainModelRequest = {
          data_query: 'SELECT feature1, feature2, target FROM training_data',
          validation_split: 0.3,
          epochs: 50,
        };

        const mockResponse: TrainingJob = {
          model_id: 'query_model',
          status: 'trained',
          metrics: {
            accuracy: 0.88,
          },
          training_time_ms: 3000,
          epochs_completed: 50,
        };

        mock.onPost('/api/v1/ml/models/query_model/train').reply(200, mockResponse);

        const result = await client.trainModel('query_model', request);

        expect(result.epochs_completed).toBe(50);
      });
    });

    describe('predict', () => {
      it('should make predictions', async () => {
        const request: PredictRequest = {
          features: [
            [1.5, 2.5, 3.5],
            [4.5, 5.5, 6.5],
          ],
          feature_names: ['feature_1', 'feature_2', 'feature_3'],
        };

        const mockResponse: Prediction = {
          predictions: [15.5, 25.5],
          confidence_scores: [0.95, 0.92],
          prediction_count: 2,
        };

        mock.onPost('/api/v1/ml/models/sales_predictor/predict').reply(200, mockResponse);

        const result = await client.predict('sales_predictor', request);

        expect(result.prediction_count).toBe(2);
        expect(result.predictions).toHaveLength(2);
        expect(result.predictions[0]).toBe(15.5);
        expect(result.confidence_scores).toBeDefined();
        expect(result.confidence_scores![0]).toBe(0.95);
      });

      it('should handle predictions without confidence scores', async () => {
        const request: PredictRequest = {
          features: [[1.0, 2.0]],
        };

        const mockResponse: Prediction = {
          predictions: [10.0],
          prediction_count: 1,
        };

        mock.onPost('/api/v1/ml/models/simple_model/predict').reply(200, mockResponse);

        const result = await client.predict('simple_model', request);

        expect(result.predictions).toHaveLength(1);
        expect(result.confidence_scores).toBeUndefined();
      });
    });

    describe('getModelMetrics', () => {
      it('should get model metrics', async () => {
        const mockResponse: ModelMetrics = {
          model_id: 'sales_predictor',
          metrics: {
            accuracy: 0.95,
            precision: 0.93,
            recall: 0.94,
            f1_score: 0.935,
          },
        };

        mock.onGet('/api/v1/ml/models/sales_predictor/metrics').reply(200, mockResponse);

        const result = await client.getModelMetrics('sales_predictor');

        expect(result.model_id).toBe('sales_predictor');
        expect(result.metrics.accuracy).toBe(0.95);
      });

      it('should get metrics with feature importance', async () => {
        const mockResponse: ModelMetrics = {
          model_id: 'tree_model',
          metrics: {
            accuracy: 0.90,
          },
          feature_importance: [
            { feature_name: 'age', importance: 0.45 },
            { feature_name: 'income', importance: 0.35 },
            { feature_name: 'location', importance: 0.20 },
          ],
        };

        mock.onGet('/api/v1/ml/models/tree_model/metrics').reply(200, mockResponse);

        const result = await client.getModelMetrics('tree_model');

        expect(result.feature_importance).toBeDefined();
        expect(result.feature_importance).toHaveLength(3);
        expect(result.feature_importance![0].feature_name).toBe('age');
      });
    });

    describe('evaluateModel', () => {
      it('should evaluate model on test data', async () => {
        const request: ModelEvaluationRequest = {
          test_features: [
            [1.0, 2.0],
            [3.0, 4.0],
          ],
          test_target: [5.0, 10.0],
          metrics: ['mse', 'rmse', 'r2'],
        };

        const mockResponse: ModelEvaluationResponse = {
          model_id: 'sales_predictor',
          metrics: {
            mse: 0.25,
            rmse: 0.5,
            r2: 0.95,
          },
        };

        mock.onPost('/api/v1/ml/models/sales_predictor/evaluate').reply(200, mockResponse);

        const result = await client.evaluateModel('sales_predictor', request);

        expect(result.metrics.mse).toBe(0.25);
        expect(result.metrics.r2).toBe(0.95);
      });

      it('should evaluate classification model with confusion matrix', async () => {
        const request: ModelEvaluationRequest = {
          test_features: [[1.0], [2.0], [3.0]],
          test_target: [0, 1, 0],
        };

        const mockResponse: ModelEvaluationResponse = {
          model_id: 'classifier',
          metrics: {
            accuracy: 0.92,
            precision: 0.90,
            recall: 0.88,
          },
          confusion_matrix: [
            [45, 5],
            [7, 43],
          ],
        };

        mock.onPost('/api/v1/ml/models/classifier/evaluate').reply(200, mockResponse);

        const result = await client.evaluateModel('classifier', request);

        expect(result.confusion_matrix).toBeDefined();
        expect(result.confusion_matrix).toHaveLength(2);
      });
    });

    describe('exportModel', () => {
      it('should export a trained model', async () => {
        const mockResponse = {
          model_id: 'sales_predictor',
          name: 'sales_predictor',
          model_type: 'LinearRegression',
          version: '1.0.0',
          created_at: Date.now(),
        };

        mock.onGet('/api/v1/ml/models/sales_predictor/export').reply(200, mockResponse);

        const result = await client.exportModel('sales_predictor');

        expect(result.model_id).toBe('sales_predictor');
        expect(result.version).toBe('1.0.0');
      });
    });
  });

  // =========================================================================
  // Analytics - OLAP Operations Tests
  // =========================================================================

  describe('Analytics - OLAP Operations', () => {
    describe('createOLAPCube', () => {
      it('should create an OLAP cube', async () => {
        const request: CreateCubeRequest = {
          name: 'sales_cube',
          dimensions: ['region', 'product', 'time'],
          measures: [
            { column: 'revenue', aggregation: 'SUM' },
            { column: 'quantity', aggregation: 'SUM' },
            { column: 'profit', aggregation: 'AVG' },
          ],
          source: 'sales_table',
        };

        const mockResponse: OLAPCube = {
          id: 'cube-123',
          name: 'sales_cube',
          dimensions: ['region', 'product', 'time'],
          measures: ['revenue', 'quantity', 'profit'],
          created_at: Date.now(),
          size_bytes: 1048576,
        };

        mock.onPost('/api/v1/analytics/olap/cubes').reply(201, mockResponse);

        const result = await client.createOLAPCube(request);

        expect(result.id).toBe('cube-123');
        expect(result.dimensions).toHaveLength(3);
        expect(result.measures).toHaveLength(3);
      });
    });

    describe('listOLAPCubes', () => {
      it('should list all OLAP cubes', async () => {
        const mockResponse = {
          cubes: [
            {
              id: 'cube-1',
              name: 'sales_cube',
              dimensions: ['region', 'time'],
              measures: ['revenue'],
              created_at: Date.now(),
              size_bytes: 500000,
            },
            {
              id: 'cube-2',
              name: 'inventory_cube',
              dimensions: ['product', 'warehouse'],
              measures: ['quantity'],
              created_at: Date.now(),
              size_bytes: 300000,
            },
          ],
          total_count: 2,
        };

        mock.onGet('/api/v1/analytics/olap/cubes').reply(200, mockResponse);

        const result = await client.listOLAPCubes();

        expect(result.total_count).toBe(2);
        expect(result.cubes).toHaveLength(2);
      });
    });

    describe('queryOLAPCube', () => {
      it('should query cube with drill-down', async () => {
        const request: CubeQueryRequest = {
          filters: { region: 'North', time: '2024-Q1' },
          operation: 'drill-down',
          target_dimension: 'product',
        };

        const mockResponse: CubeQueryResponse = {
          results: [
            { product: 'Widget A', revenue: 10000, quantity: 100 },
            { product: 'Widget B', revenue: 15000, quantity: 150 },
          ],
          row_count: 2,
          execution_time_ms: 45,
        };

        mock.onPost('/api/v1/analytics/olap/cubes/sales_cube/query').reply(200, mockResponse);

        const result = await client.queryOLAPCube('sales_cube', request);

        expect(result.row_count).toBe(2);
        expect(result.results).toHaveLength(2);
        expect(result.execution_time_ms).toBe(45);
      });

      it('should query cube with roll-up', async () => {
        const request: CubeQueryRequest = {
          filters: {},
          operation: 'roll-up',
          target_dimension: 'region',
        };

        const mockResponse: CubeQueryResponse = {
          results: [
            { region: 'All', total_revenue: 100000 },
          ],
          row_count: 1,
          execution_time_ms: 30,
        };

        mock.onPost('/api/v1/analytics/olap/cubes/sales_cube/query').reply(200, mockResponse);

        const result = await client.queryOLAPCube('sales_cube', request);

        expect(result.row_count).toBe(1);
      });

      it('should query cube with slice', async () => {
        const request: CubeQueryRequest = {
          filters: { time: '2024-Q1' },
          operation: 'slice',
        };

        const mockResponse: CubeQueryResponse = {
          results: [
            { region: 'North', product: 'A', revenue: 5000 },
            { region: 'South', product: 'B', revenue: 7000 },
          ],
          row_count: 2,
          execution_time_ms: 25,
        };

        mock.onPost('/api/v1/analytics/olap/cubes/sales_cube/query').reply(200, mockResponse);

        const result = await client.queryOLAPCube('sales_cube', request);

        expect(result.results).toHaveLength(2);
      });
    });

    describe('deleteOLAPCube', () => {
      it('should delete an OLAP cube', async () => {
        mock.onDelete('/api/v1/analytics/olap/cubes/old_cube').reply(204);

        await expect(client.deleteOLAPCube('old_cube')).resolves.not.toThrow();
      });
    });
  });

  // =========================================================================
  // Analytics - Query Statistics Tests
  // =========================================================================

  describe('Analytics - Query Statistics & Workload', () => {
    describe('getQueryStatistics', () => {
      it('should get query statistics with filters', async () => {
        const filter: QueryStatsFilter = {
          time_range_hours: 24,
          min_execution_time_ms: 1000,
          limit: 50,
        };

        const mockResponse: QueryStatisticsResponse = {
          statistics: [
            {
              query_id: 1,
              normalized_sql: 'SELECT * FROM users WHERE id = ?',
              execution_count: 150,
              avg_execution_time_ms: 25.5,
              min_execution_time_ms: 10,
              max_execution_time_ms: 100,
              total_rows_examined: 15000,
              total_rows_returned: 150,
              last_executed: Date.now(),
            },
          ],
          total_queries: 1000,
          avg_execution_time_ms: 45.2,
          slow_query_count: 15,
        };

        mock.onGet('/api/v1/analytics/query-stats').reply(200, mockResponse);

        const result = await client.getQueryStatistics(filter);

        expect(result.total_queries).toBe(1000);
        expect(result.slow_query_count).toBe(15);
        expect(result.statistics).toHaveLength(1);
      });

      it('should get query statistics without filters', async () => {
        const mockResponse: QueryStatisticsResponse = {
          statistics: [],
          total_queries: 500,
          avg_execution_time_ms: 30.0,
          slow_query_count: 5,
        };

        mock.onGet('/api/v1/analytics/query-stats').reply(200, mockResponse);

        const result = await client.getQueryStatistics();

        expect(result.total_queries).toBe(500);
      });
    });

    describe('analyzeWorkload', () => {
      it('should analyze workload and provide recommendations', async () => {
        const mockResponse: WorkloadAnalysisResponse = {
          analysis_timestamp: Date.now(),
          total_queries: 5000,
          unique_patterns: 250,
          recommendations: [
            {
              recommendation_type: 'INDEX',
              priority: 'HIGH',
              description: 'Create index on users.email',
              affected_tables: ['users'],
              affected_columns: ['email'],
              estimated_improvement: 0.75,
            },
            {
              recommendation_type: 'PARTITION',
              priority: 'MEDIUM',
              description: 'Partition orders table by date',
              affected_tables: ['orders'],
              affected_columns: ['order_date'],
              estimated_improvement: 0.50,
            },
          ],
          top_queries: [
            {
              query_id: 1,
              normalized_sql: 'SELECT * FROM products WHERE category = ?',
              execution_count: 1000,
              avg_execution_time_ms: 50.0,
              min_execution_time_ms: 20,
              max_execution_time_ms: 200,
              total_rows_examined: 100000,
              total_rows_returned: 10000,
              last_executed: Date.now(),
            },
          ],
          table_access_patterns: {
            users: 2500,
            products: 1500,
            orders: 1000,
          },
        };

        mock.onGet('/api/v1/analytics/workload').reply(200, mockResponse);

        const result = await client.analyzeWorkload();

        expect(result.total_queries).toBe(5000);
        expect(result.recommendations).toHaveLength(2);
        expect(result.recommendations[0].priority).toBe('HIGH');
        expect(result.top_queries).toHaveLength(1);
        expect(result.table_access_patterns.users).toBe(2500);
      });
    });

    describe('getRecommendations', () => {
      it('should get optimization recommendations', async () => {
        const mockResponse: RecommendationEntry[] = [
          {
            recommendation_type: 'INDEX',
            priority: 'HIGH',
            description: 'Create index on frequently queried columns',
            affected_tables: ['users'],
            affected_columns: ['email'],
            estimated_improvement: 0.75,
          },
          {
            recommendation_type: 'QUERY_REWRITE',
            priority: 'MEDIUM',
            description: 'Optimize JOIN order',
            affected_tables: ['orders', 'customers'],
            affected_columns: [],
            estimated_improvement: 0.30,
          },
        ];

        mock.onGet('/api/v1/analytics/recommendations').reply(200, mockResponse);

        const result = await client.getRecommendations();

        expect(result).toHaveLength(2);
        expect(result[0].recommendation_type).toBe('INDEX');
        expect(result[1].priority).toBe('MEDIUM');
      });
    });
  });

  // =========================================================================
  // Analytics - Data Quality Tests
  // =========================================================================

  describe('Analytics - Data Quality', () => {
    describe('profileTable', () => {
      it('should profile a table', async () => {
        const request: ProfileTableRequest = {
          sample_percent: 10,
          include_patterns: true,
          suggest_indexes: true,
        };

        const mockResponse: ProfileTableResponse = {
          table_name: 'customers',
          row_count: 100000,
          column_profiles: [
            {
              column_name: 'id',
              inferred_type: 'INTEGER',
              null_count: 0,
              null_percentage: 0.0,
              distinct_count: 100000,
              cardinality: 1.0,
            },
            {
              column_name: 'email',
              inferred_type: 'STRING',
              null_count: 5,
              null_percentage: 0.005,
              distinct_count: 99995,
              cardinality: 0.99995,
              avg_length: 25.5,
            },
          ],
          index_suggestions: [
            {
              index_type: 'BTREE',
              columns: ['email'],
              reason: 'High cardinality, frequently queried',
              estimated_benefit: 'HIGH',
            },
          ],
          profiled_at: Date.now(),
        };

        mock.onPost('/api/v1/analytics/profile/customers').reply(200, mockResponse);

        const result = await client.profileTable('customers', request);

        expect(result.table_name).toBe('customers');
        expect(result.row_count).toBe(100000);
        expect(result.column_profiles).toHaveLength(2);
        expect(result.index_suggestions).toHaveLength(1);
      });
    });

    describe('getQualityMetrics', () => {
      it('should get data quality metrics', async () => {
        const mockResponse: QualityMetrics = {
          table_name: 'customers',
          overall_score: 0.92,
          completeness: 0.98,
          uniqueness: 0.95,
          validity: 0.90,
          consistency: 0.88,
          accuracy: 0.93,
          timeliness: 0.87,
          row_count: 100000,
          issue_count: 150,
          analyzed_at: Date.now(),
        };

        mock.onGet('/api/v1/analytics/quality/customers').reply(200, mockResponse);

        const result = await client.getQualityMetrics('customers');

        expect(result.table_name).toBe('customers');
        expect(result.overall_score).toBe(0.92);
        expect(result.completeness).toBe(0.98);
        expect(result.issue_count).toBe(150);
      });
    });

    describe('getQualityIssues', () => {
      it('should get data quality issues', async () => {
        const mockResponse: QualityIssuesResponse = {
          table_name: 'customers',
          issues: [
            {
              issue_type: 'NULL_VALUE',
              severity: 'CRITICAL',
              column_name: 'email',
              row_number: 12345,
              description: 'Email field is null',
              suggested_fix: 'Require email during registration',
            },
            {
              issue_type: 'DUPLICATE',
              severity: 'WARNING',
              column_name: 'phone',
              description: 'Duplicate phone numbers found',
              suggested_fix: 'Add unique constraint',
            },
          ],
          total_count: 25,
          critical_count: 5,
          warning_count: 20,
        };

        mock.onGet('/api/v1/analytics/quality/customers/issues').reply(200, mockResponse);

        const result = await client.getQualityIssues('customers');

        expect(result.table_name).toBe('customers');
        expect(result.total_count).toBe(25);
        expect(result.critical_count).toBe(5);
        expect(result.issues).toHaveLength(2);
        expect(result.issues[0].severity).toBe('CRITICAL');
      });
    });
  });

  // =========================================================================
  // Analytics - Materialized Views Tests
  // =========================================================================

  describe('Analytics - Materialized Views', () => {
    describe('createMaterializedView', () => {
      it('should create a materialized view', async () => {
        const request: CreateMaterializedViewRequest = {
          name: 'monthly_sales',
          query: 'SELECT DATE_TRUNC(\'month\', date) as month, SUM(revenue) as total FROM sales GROUP BY month',
          refresh_schedule: {
            interval_secs: 3600,
            auto_refresh: true,
          },
          indexes: ['month'],
        };

        const mockResponse: MaterializedView = {
          id: 'mv-123',
          name: 'monthly_sales',
          query: request.query,
          row_count: 24,
          last_refreshed: Date.now(),
          next_refresh: Date.now() + 3600000,
          size_bytes: 2048,
          indexes: ['month'],
        };

        mock.onPost('/api/v1/analytics/materialized-views').reply(201, mockResponse);

        const result = await client.createMaterializedView(request);

        expect(result.id).toBe('mv-123');
        expect(result.name).toBe('monthly_sales');
        expect(result.indexes).toContain('month');
      });
    });

    describe('listMaterializedViews', () => {
      it('should list all materialized views', async () => {
        const mockResponse = {
          views: [
            {
              id: 'mv-1',
              name: 'daily_stats',
              query: 'SELECT * FROM stats',
              row_count: 365,
              last_refreshed: Date.now(),
              size_bytes: 10240,
              indexes: [],
            },
          ],
          total_count: 1,
        };

        mock.onGet('/api/v1/analytics/materialized-views').reply(200, mockResponse);

        const result = await client.listMaterializedViews();

        expect(result.total_count).toBe(1);
        expect(result.views).toHaveLength(1);
      });
    });

    describe('refreshMaterializedView', () => {
      it('should refresh a materialized view', async () => {
        const mockResponse: RefreshMaterializedViewResponse = {
          view_id: 'mv-123',
          view_name: 'monthly_sales',
          rows_refreshed: 24,
          refresh_time_ms: 150,
          refreshed_at: Date.now(),
        };

        mock.onPost('/api/v1/analytics/materialized-views/mv-123/refresh').reply(200, mockResponse);

        const result = await client.refreshMaterializedView('mv-123');

        expect(result.view_id).toBe('mv-123');
        expect(result.rows_refreshed).toBe(24);
        expect(result.refresh_time_ms).toBe(150);
      });
    });
  });

  // =========================================================================
  // In-Memory Column Store Tests
  // =========================================================================

  describe('In-Memory Column Store', () => {
    describe('enableInMemory', () => {
      it('should enable in-memory for a table', async () => {
        const request: EnableInMemoryRequest = {
          table: 'hot_data',
          columns: ['id', 'value', 'timestamp'],
          priority: 'high',
          compression: true,
        };

        const mockResponse: EnableInMemoryResponse = {
          table: 'hot_data',
          status: 'enabled',
          population_started: true,
          estimated_size_bytes: 10485760,
        };

        mock.onPost('/api/v1/inmemory/enable').reply(200, mockResponse);

        const result = await client.enableInMemory(request);

        expect(result.table).toBe('hot_data');
        expect(result.population_started).toBe(true);
        expect(result.estimated_size_bytes).toBe(10485760);
      });

      it('should enable with default options', async () => {
        const request: EnableInMemoryRequest = {
          table: 'simple_table',
        };

        const mockResponse: EnableInMemoryResponse = {
          table: 'simple_table',
          status: 'enabled',
          population_started: true,
          estimated_size_bytes: 1048576,
        };

        mock.onPost('/api/v1/inmemory/enable').reply(200, mockResponse);

        const result = await client.enableInMemory(request);

        expect(result.table).toBe('simple_table');
      });
    });

    describe('disableInMemory', () => {
      it('should disable in-memory for a table', async () => {
        mock.onPost('/api/v1/inmemory/disable').reply(200);

        await expect(client.disableInMemory('old_table')).resolves.not.toThrow();
      });
    });

    describe('getInMemoryStatus', () => {
      it('should get in-memory status', async () => {
        const mockResponse: InMemoryStatusResponse = {
          enabled: true,
          total_memory_bytes: 8589934592,
          used_memory_bytes: 4294967296,
          memory_utilization_percent: 50.0,
          tables: [
            {
              table_name: 'hot_data',
              memory_bytes: 2147483648,
              row_count: 1000000,
              compression_ratio: 3.5,
              population_status: 'populated',
              last_accessed: Date.now(),
            },
          ],
        };

        mock.onGet('/api/v1/inmemory/status').reply(200, mockResponse);

        const result = await client.getInMemoryStatus();

        expect(result.enabled).toBe(true);
        expect(result.memory_utilization_percent).toBe(50.0);
        expect(result.tables).toHaveLength(1);
      });
    });

    describe('getInMemoryStats', () => {
      it('should get in-memory statistics', async () => {
        const mockResponse: InMemoryStats = {
          total_stores: 5,
          total_memory_bytes: 4294967296,
          max_memory_bytes: 8589934592,
          memory_pressure: 0.5,
          cache_hits: 1000000,
          cache_misses: 50000,
          cache_hit_ratio: 0.95,
          population_queue_size: 2,
        };

        mock.onGet('/api/v1/inmemory/stats').reply(200, mockResponse);

        const result = await client.getInMemoryStats();

        expect(result.total_stores).toBe(5);
        expect(result.cache_hit_ratio).toBe(0.95);
        expect(result.memory_pressure).toBe(0.5);
      });
    });

    describe('populateTable', () => {
      it('should populate a table into memory', async () => {
        const request: PopulateRequest = {
          table: 'products',
          force: true,
          strategy: 'full',
        };

        const mockResponse: PopulateResponse = {
          table: 'products',
          status: 'populated',
          rows_populated: 50000,
          duration_ms: 2500,
        };

        mock.onPost('/api/v1/inmemory/populate').reply(200, mockResponse);

        const result = await client.populateTable(request);

        expect(result.table).toBe('products');
        expect(result.rows_populated).toBe(50000);
        expect(result.duration_ms).toBe(2500);
      });

      it('should populate with incremental strategy', async () => {
        const request: PopulateRequest = {
          table: 'events',
          strategy: 'incremental',
        };

        const mockResponse: PopulateResponse = {
          table: 'events',
          status: 'populated',
          rows_populated: 1000,
          duration_ms: 100,
        };

        mock.onPost('/api/v1/inmemory/populate').reply(200, mockResponse);

        const result = await client.populateTable(request);

        expect(result.rows_populated).toBe(1000);
      });
    });

    describe('evictTables', () => {
      it('should evict specific table', async () => {
        const request: EvictRequest = {
          table: 'old_data',
        };

        const mockResponse: EvictResponse = {
          tables_evicted: ['old_data'],
          memory_freed_bytes: 1073741824,
        };

        mock.onPost('/api/v1/inmemory/evict').reply(200, mockResponse);

        const result = await client.evictTables(request);

        expect(result.tables_evicted).toContain('old_data');
        expect(result.memory_freed_bytes).toBe(1073741824);
      });

      it('should evict based on memory threshold', async () => {
        const request: EvictRequest = {
          threshold_percent: 80,
        };

        const mockResponse: EvictResponse = {
          tables_evicted: ['table1', 'table2'],
          memory_freed_bytes: 2147483648,
        };

        mock.onPost('/api/v1/inmemory/evict').reply(200, mockResponse);

        const result = await client.evictTables(request);

        expect(result.tables_evicted).toHaveLength(2);
        expect(result.memory_freed_bytes).toBeGreaterThan(0);
      });
    });

    describe('getTableStatus', () => {
      it('should get table population status', async () => {
        const mockResponse: InMemoryTableInfo = {
          table_name: 'products',
          memory_bytes: 536870912,
          row_count: 50000,
          compression_ratio: 4.2,
          population_status: 'populated',
          last_accessed: Date.now(),
        };

        mock.onGet('/api/v1/inmemory/tables/products/status').reply(200, mockResponse);

        const result = await client.getTableStatus('products');

        expect(result.table_name).toBe('products');
        expect(result.compression_ratio).toBe(4.2);
        expect(result.population_status).toBe('populated');
      });
    });

    describe('compactMemory', () => {
      it('should trigger memory compaction', async () => {
        mock.onPost('/api/v1/inmemory/compact').reply(200);

        await expect(client.compactMemory()).resolves.not.toThrow();
      });
    });

    describe('updateInMemoryConfig', () => {
      it('should update in-memory configuration', async () => {
        const config = {
          max_memory_bytes: 16 * 1024 * 1024 * 1024,
          auto_populate: true,
        };

        mock.onPut('/api/v1/inmemory/config').reply(200);

        await expect(client.updateInMemoryConfig(config)).resolves.not.toThrow();
      });
    });

    describe('getInMemoryConfig', () => {
      it('should get in-memory configuration', async () => {
        const mockResponse: InMemoryConfig = {
          max_memory_bytes: 8589934592,
          auto_populate: true,
          enable_compression: true,
          vector_width: 8,
          cache_line_size: 64,
        };

        mock.onGet('/api/v1/inmemory/config').reply(200, mockResponse);

        const result = await client.getInMemoryConfig();

        expect(result.max_memory_bytes).toBe(8589934592);
        expect(result.vector_width).toBe(8);
        expect(result.enable_compression).toBe(true);
      });
    });
  });
});
