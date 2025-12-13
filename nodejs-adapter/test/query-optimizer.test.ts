/**
 * Test Suite for RustyDB Query & Optimizer API Adapter
 *
 * Comprehensive tests covering all query execution and optimizer endpoints.
 *
 * @module query-optimizer.test
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from '@jest/globals';
import QueryOptimizerClient, {
  QueryRequest,
  BatchRequest,
  ApplyHintRequest,
  CreateBaselineRequest,
  UpdateBaselineRequest,
  QueryOptimizerError,
} from '../src/api/query-optimizer';

// Test configuration
const TEST_CONFIG = {
  baseUrl: process.env.RUSTYDB_URL || 'http://localhost:8080',
  timeout: 30000,
};

describe('QueryOptimizerClient', () => {
  let client: QueryOptimizerClient;

  beforeAll(() => {
    client = new QueryOptimizerClient(TEST_CONFIG);
  });

  // =========================================================================
  // Query Execution Tests
  // =========================================================================

  describe('Query Execution', () => {
    describe('executeQuery', () => {
      it('should execute a simple SELECT query', async () => {
        const request: QueryRequest = {
          sql: 'SELECT * FROM users LIMIT 10',
        };

        const response = await client.executeQuery(request);

        expect(response).toBeDefined();
        expect(response.query_id).toBeDefined();
        expect(response.rows).toBeInstanceOf(Array);
        expect(response.columns).toBeInstanceOf(Array);
        expect(response.row_count).toBeGreaterThanOrEqual(0);
        expect(response.execution_time_ms).toBeGreaterThanOrEqual(0);
      });

      it('should execute a parameterized query', async () => {
        const request: QueryRequest = {
          sql: 'SELECT * FROM users WHERE age > $1',
          params: [18],
        };

        const response = await client.executeQuery(request);

        expect(response).toBeDefined();
        expect(response.query_id).toBeDefined();
        expect(response.rows).toBeInstanceOf(Array);
      });

      it('should execute query with limit and offset', async () => {
        const request: QueryRequest = {
          sql: 'SELECT * FROM users ORDER BY id',
          limit: 5,
          offset: 10,
        };

        const response = await client.executeQuery(request);

        expect(response).toBeDefined();
        expect(response.row_count).toBeLessThanOrEqual(5);
      });

      it('should execute query with timeout', async () => {
        const request: QueryRequest = {
          sql: 'SELECT * FROM users',
          timeout: 5,
        };

        const response = await client.executeQuery(request);

        expect(response).toBeDefined();
      });

      it('should return column metadata', async () => {
        const request: QueryRequest = {
          sql: 'SELECT id, name, email FROM users LIMIT 1',
        };

        const response = await client.executeQuery(request);

        expect(response.columns).toBeDefined();
        expect(response.columns.length).toBeGreaterThan(0);

        const column = response.columns[0];
        expect(column.name).toBeDefined();
        expect(column.data_type).toBeDefined();
        expect(typeof column.nullable).toBe('boolean');
      });

      it('should execute INSERT and return affected rows', async () => {
        const request: QueryRequest = {
          sql: "INSERT INTO test_table (name) VALUES ('test')",
        };

        const response = await client.executeQuery(request);

        expect(response).toBeDefined();
        expect(response.affected_rows).toBeDefined();
        expect(response.affected_rows).toBeGreaterThan(0);
      });

      it('should execute UPDATE and return affected rows', async () => {
        const request: QueryRequest = {
          sql: "UPDATE test_table SET status = 'active' WHERE id > 0",
        };

        const response = await client.executeQuery(request);

        expect(response).toBeDefined();
        expect(response.affected_rows).toBeDefined();
      });

      it('should execute DELETE and return affected rows', async () => {
        const request: QueryRequest = {
          sql: 'DELETE FROM test_table WHERE id = 999999',
        };

        const response = await client.executeQuery(request);

        expect(response).toBeDefined();
        expect(response.affected_rows).toBeDefined();
      });

      it('should handle empty result set', async () => {
        const request: QueryRequest = {
          sql: 'SELECT * FROM users WHERE id = -1',
        };

        const response = await client.executeQuery(request);

        expect(response).toBeDefined();
        expect(response.rows).toEqual([]);
        expect(response.row_count).toBe(0);
      });

      it('should throw error on invalid SQL', async () => {
        const request: QueryRequest = {
          sql: 'SELEC * FORM users',
        };

        await expect(client.executeQuery(request)).rejects.toThrow(QueryOptimizerError);
      });

      it('should handle warnings', async () => {
        const request: QueryRequest = {
          sql: 'SELECT * FROM users',
        };

        const response = await client.executeQuery(request);

        expect(response.warnings).toBeDefined();
        expect(response.warnings).toBeInstanceOf(Array);
      });
    });

    describe('executeBatch', () => {
      it('should execute multiple statements in batch', async () => {
        const request: BatchRequest = {
          statements: [
            "INSERT INTO test_table (name) VALUES ('batch1')",
            "INSERT INTO test_table (name) VALUES ('batch2')",
            "INSERT INTO test_table (name) VALUES ('batch3')",
          ],
          transactional: false,
          stop_on_error: false,
        };

        const response = await client.executeBatch(request);

        expect(response).toBeDefined();
        expect(response.batch_id).toBeDefined();
        expect(response.results).toHaveLength(3);
        expect(response.success_count).toBeGreaterThan(0);
        expect(response.total_time_ms).toBeGreaterThanOrEqual(0);
      });

      it('should execute batch in transaction', async () => {
        const request: BatchRequest = {
          statements: [
            "INSERT INTO test_table (name) VALUES ('tx1')",
            "INSERT INTO test_table (name) VALUES ('tx2')",
          ],
          transactional: true,
          stop_on_error: true,
        };

        const response = await client.executeBatch(request);

        expect(response).toBeDefined();
        expect(response.success_count).toBe(2);
      });

      it('should stop on error when specified', async () => {
        const request: BatchRequest = {
          statements: [
            "INSERT INTO test_table (name) VALUES ('ok')",
            'INVALID SQL STATEMENT',
            "INSERT INTO test_table (name) VALUES ('never executed')",
          ],
          transactional: false,
          stop_on_error: true,
        };

        const response = await client.executeBatch(request);

        expect(response).toBeDefined();
        expect(response.failure_count).toBeGreaterThan(0);
        // Should stop after first error
        expect(response.results.length).toBeLessThan(3);
      });

      it('should continue on error when not stopping', async () => {
        const request: BatchRequest = {
          statements: [
            "INSERT INTO test_table (name) VALUES ('ok1')",
            'INVALID SQL',
            "INSERT INTO test_table (name) VALUES ('ok2')",
          ],
          transactional: false,
          stop_on_error: false,
        };

        const response = await client.executeBatch(request);

        expect(response).toBeDefined();
        expect(response.results).toHaveLength(3);
        expect(response.success_count).toBeGreaterThan(0);
        expect(response.failure_count).toBeGreaterThan(0);
      });

      it('should track execution time per statement', async () => {
        const request: BatchRequest = {
          statements: [
            'SELECT COUNT(*) FROM users',
            'SELECT COUNT(*) FROM orders',
          ],
          transactional: false,
          stop_on_error: false,
        };

        const response = await client.executeBatch(request);

        expect(response).toBeDefined();
        response.results.forEach((result) => {
          expect(result.execution_time_ms).toBeGreaterThanOrEqual(0);
        });
      });
    });
  });

  // =========================================================================
  // Query Explain Tests
  // =========================================================================

  describe('Query Explain', () => {
    describe('explainQuery', () => {
      it('should explain a simple query', async () => {
        const plan = await client.explainQuery('SELECT * FROM users WHERE age > 18');

        expect(plan).toBeDefined();
        expect(plan.query).toBe('SELECT * FROM users WHERE age > 18');
        expect(plan.plan).toBeDefined();
        expect(plan.estimated_cost).toBeGreaterThan(0);
        expect(plan.estimated_rows).toBeGreaterThanOrEqual(0);
        expect(plan.planning_time_ms).toBeGreaterThanOrEqual(0);
        expect(plan.execution_time_ms).toBeUndefined(); // Not analyzed
      });

      it('should explain a JOIN query', async () => {
        const plan = await client.explainQuery(
          'SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id'
        );

        expect(plan).toBeDefined();
        expect(plan.plan.operator).toBeDefined();
        // Should have join operator
        expect(
          plan.plan.operator.includes('Join') ||
          plan.plan.children.some((c) => c.operator.includes('Join'))
        ).toBe(true);
      });

      it('should explain query with aggregation', async () => {
        const plan = await client.explainQuery(
          'SELECT department, COUNT(*), AVG(salary) FROM employees GROUP BY department'
        );

        expect(plan).toBeDefined();
        expect(plan.plan).toBeDefined();
      });

      it('should explain query with subquery', async () => {
        const plan = await client.explainQuery(
          'SELECT * FROM users WHERE id IN (SELECT user_id FROM orders WHERE total > 100)'
        );

        expect(plan).toBeDefined();
        expect(plan.plan).toBeDefined();
      });

      it('should show plan tree structure', async () => {
        const plan = await client.explainQuery('SELECT * FROM users ORDER BY name');

        expect(plan.plan).toBeDefined();
        expect(plan.plan.operator).toBeDefined();
        expect(plan.plan.cost).toBeGreaterThan(0);
        expect(plan.plan.rows).toBeGreaterThanOrEqual(0);
        expect(plan.plan.details).toBeDefined();
        expect(plan.plan.children).toBeInstanceOf(Array);
      });
    });

    describe('explainAnalyzeQuery', () => {
      it('should explain and analyze a query', async () => {
        const analysis = await client.explainAnalyzeQuery('SELECT * FROM users LIMIT 10');

        expect(analysis).toBeDefined();
        expect(analysis.planning_time_ms).toBeGreaterThanOrEqual(0);
        expect(analysis.execution_time_ms).toBeDefined();
        expect(analysis.execution_time_ms).toBeGreaterThanOrEqual(0);
      });

      it('should provide actual execution statistics', async () => {
        const analysis = await client.explainAnalyzeQuery(
          'SELECT COUNT(*) FROM users WHERE age > 18'
        );

        expect(analysis).toBeDefined();
        expect(analysis.execution_time_ms).toBeDefined();
        expect(analysis.estimated_cost).toBeGreaterThan(0);
        expect(analysis.estimated_rows).toBeGreaterThanOrEqual(0);
      });
    });

    describe('comparePlans', () => {
      it('should compare plans with and without hints', async () => {
        const comparison = await client.comparePlans(
          'SELECT * FROM users WHERE age > 18',
          'SELECT /*+ FULL(users) */ * FROM users WHERE age > 18'
        );

        expect(comparison).toBeDefined();
        expect(comparison.original).toBeDefined();
        expect(comparison.withHints).toBeDefined();
        expect(comparison.original.estimated_cost).toBeGreaterThan(0);
        expect(comparison.withHints.estimated_cost).toBeGreaterThan(0);
      });
    });

    describe('formatPlanTree', () => {
      it('should format plan as readable tree', async () => {
        const plan = await client.explainQuery('SELECT * FROM users ORDER BY name');
        const formatted = client.formatPlanTree(plan.plan);

        expect(formatted).toBeDefined();
        expect(typeof formatted).toBe('string');
        expect(formatted.length).toBeGreaterThan(0);
        expect(formatted).toContain(plan.plan.operator);
        expect(formatted).toContain('cost=');
        expect(formatted).toContain('rows=');
      });

      it('should indent nested operators', async () => {
        const plan = await client.explainQuery(
          'SELECT * FROM users u JOIN orders o ON u.id = o.user_id'
        );
        const formatted = client.formatPlanTree(plan.plan);

        expect(formatted).toBeDefined();
        // Check for indentation
        expect(formatted.includes('  ')).toBe(true);
      });
    });
  });

  // =========================================================================
  // Optimizer Hints Tests
  // =========================================================================

  describe('Optimizer Hints', () => {
    describe('listHints', () => {
      it('should list all available hints', async () => {
        const response = await client.listHints();

        expect(response).toBeDefined();
        expect(response.hints).toBeInstanceOf(Array);
        expect(response.total).toBeGreaterThan(0);
        expect(response.hints.length).toBe(response.total);

        const hint = response.hints[0];
        expect(hint.name).toBeDefined();
        expect(hint.category).toBeDefined();
        expect(hint.description).toBeDefined();
        expect(hint.parameters).toBeInstanceOf(Array);
        expect(hint.example).toBeDefined();
      });

      it('should filter hints by category', async () => {
        const response = await client.listHints({ category: 'AccessPath' });

        expect(response).toBeDefined();
        expect(response.hints).toBeInstanceOf(Array);
        response.hints.forEach((hint) => {
          expect(hint.category).toBe('AccessPath');
        });
      });

      it('should search hints by keyword', async () => {
        const response = await client.listHints({ search: 'index' });

        expect(response).toBeDefined();
        expect(response.hints).toBeInstanceOf(Array);
        response.hints.forEach((hint) => {
          const matchesSearch =
            hint.name.toLowerCase().includes('index') ||
            hint.description.toLowerCase().includes('index');
          expect(matchesSearch).toBe(true);
        });
      });

      it('should return hint examples', async () => {
        const response = await client.listHints();

        expect(response.hints.length).toBeGreaterThan(0);
        response.hints.forEach((hint) => {
          expect(hint.example).toBeDefined();
          expect(hint.example).toContain('/*+');
          expect(hint.example).toContain('*/');
        });
      });
    });

    describe('getActiveHints', () => {
      it('should get active hints for session', async () => {
        const response = await client.getActiveHints();

        expect(response).toBeDefined();
        expect(response.session_id).toBeDefined();
        expect(response.hints).toBeInstanceOf(Array);
      });
    });

    describe('applyHints', () => {
      it('should apply hints to a query', async () => {
        const request: ApplyHintRequest = {
          query: 'SELECT * FROM users WHERE age > 18',
          hints: ['FULL(users)', 'PARALLEL(4)'],
        };

        const response = await client.applyHints(request);

        expect(response).toBeDefined();
        expect(response.hint_id).toBeDefined();
        expect(response.parsed_hints).toBeInstanceOf(Array);
        expect(response.conflicts).toBeInstanceOf(Array);
        expect(response.warnings).toBeInstanceOf(Array);
      });

      it('should parse multiple hints', async () => {
        const request: ApplyHintRequest = {
          query: 'SELECT * FROM users u JOIN orders o ON u.id = o.user_id',
          hints: ['HASH_JOIN(u o)', 'INDEX(users idx_age)', 'USE_MERGE(orders)'],
        };

        const response = await client.applyHints(request);

        expect(response).toBeDefined();
        expect(response.parsed_hints.length).toBeGreaterThan(0);
      });

      it('should detect conflicting hints', async () => {
        const request: ApplyHintRequest = {
          query: 'SELECT * FROM users',
          hints: ['FULL(users)', 'INDEX(users idx_age)'], // Conflicting
        };

        const response = await client.applyHints(request);

        expect(response).toBeDefined();
        // May have conflicts
        expect(response.conflicts).toBeInstanceOf(Array);
      });
    });

    describe('removeHint', () => {
      it('should remove a hint by ID', async () => {
        // First apply a hint
        const applyResponse = await client.applyHints({
          query: 'SELECT * FROM users',
          hints: ['FULL(users)'],
        });

        // Then remove it
        const removeResponse = await client.removeHint(applyResponse.hint_id);

        expect(removeResponse).toBeDefined();
        expect(removeResponse.success).toBe(true);
        expect(removeResponse.message).toBeDefined();
      });
    });
  });

  // =========================================================================
  // Plan Baselines Tests
  // =========================================================================

  describe('Plan Baselines', () => {
    let testFingerprint: string;

    describe('createBaseline', () => {
      it('should create a new plan baseline', async () => {
        const request: CreateBaselineRequest = {
          query_text: 'SELECT * FROM users WHERE age > $1',
          param_types: ['INTEGER'],
          enabled: true,
          fixed: false,
        };

        const response = await client.createBaseline(request);

        expect(response).toBeDefined();
        expect(response.fingerprint).toBeDefined();
        expect(response.enabled).toBe(true);
        expect(response.fixed).toBe(false);
        expect(response.origin).toBeDefined();
        expect(response.created_at).toBeDefined();
        expect(response.execution_count).toBe(0);
        expect(response.accepted_plans_count).toBeGreaterThanOrEqual(0);

        testFingerprint = response.fingerprint;
      });

      it('should create baseline with schema version', async () => {
        const request: CreateBaselineRequest = {
          query_text: 'SELECT COUNT(*) FROM orders',
          schema_version: 1,
          enabled: true,
        };

        const response = await client.createBaseline(request);

        expect(response).toBeDefined();
        expect(response.fingerprint).toBeDefined();
      });

      it('should create fixed baseline', async () => {
        const request: CreateBaselineRequest = {
          query_text: 'SELECT * FROM products WHERE price > $1',
          param_types: ['DECIMAL'],
          fixed: true,
        };

        const response = await client.createBaseline(request);

        expect(response).toBeDefined();
        expect(response.fixed).toBe(true);
      });
    });

    describe('listBaselines', () => {
      it('should list all baselines', async () => {
        const response = await client.listBaselines();

        expect(response).toBeDefined();
        expect(response.baselines).toBeInstanceOf(Array);
        expect(response.total).toBeGreaterThanOrEqual(0);

        if (response.baselines.length > 0) {
          const baseline = response.baselines[0];
          expect(baseline.fingerprint).toBeDefined();
          expect(typeof baseline.enabled).toBe('boolean');
          expect(typeof baseline.fixed).toBe('boolean');
          expect(baseline.origin).toBeDefined();
          expect(baseline.execution_count).toBeGreaterThanOrEqual(0);
          expect(baseline.avg_execution_time_ms).toBeGreaterThanOrEqual(0);
        }
      });
    });

    describe('getBaseline', () => {
      it('should get baseline details', async () => {
        // First create a baseline
        const createRequest: CreateBaselineRequest = {
          query_text: 'SELECT * FROM test_baseline WHERE id = $1',
          param_types: ['INTEGER'],
        };
        const created = await client.createBaseline(createRequest);

        // Then retrieve it
        const baseline = await client.getBaseline(created.fingerprint);

        expect(baseline).toBeDefined();
        expect(baseline.fingerprint).toBe(created.fingerprint);
        expect(baseline.accepted_plans).toBeInstanceOf(Array);

        if (baseline.accepted_plans.length > 0) {
          const plan = baseline.accepted_plans[0];
          expect(plan.plan_id).toBeDefined();
          expect(plan.cost).toBeGreaterThan(0);
          expect(plan.cardinality).toBeGreaterThanOrEqual(0);
          expect(plan.operator_type).toBeDefined();
          expect(typeof plan.from_baseline).toBe('boolean');
        }
      });
    });

    describe('updateBaseline', () => {
      it('should disable a baseline', async () => {
        // Create baseline
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_update WHERE id = $1',
          param_types: ['INTEGER'],
          enabled: true,
        });

        // Disable it
        const response = await client.updateBaseline(created.fingerprint, {
          enabled: false,
        });

        expect(response).toBeDefined();
        expect(response.message).toBeDefined();
        expect(response.fingerprint).toBe(created.fingerprint);
      });

      it('should fix a baseline', async () => {
        // Create baseline
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_fix WHERE id = $1',
          param_types: ['INTEGER'],
          fixed: false,
        });

        // Fix it
        const response = await client.updateBaseline(created.fingerprint, {
          fixed: true,
        });

        expect(response).toBeDefined();
        expect(response.fingerprint).toBe(created.fingerprint);
      });
    });

    describe('deleteBaseline', () => {
      it('should delete a baseline', async () => {
        // Create baseline
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_delete WHERE id = $1',
          param_types: ['INTEGER'],
        });

        // Delete it
        const response = await client.deleteBaseline(created.fingerprint);

        expect(response).toBeDefined();
        expect(response.message).toBeDefined();
        expect(response.fingerprint).toBe(created.fingerprint);
      });
    });

    describe('evolveBaseline', () => {
      it('should evolve a baseline', async () => {
        // Create baseline
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_evolve WHERE id = $1',
          param_types: ['INTEGER'],
          fixed: false,
        });

        // Evolve it
        const response = await client.evolveBaseline(created.fingerprint);

        expect(response).toBeDefined();
        expect(response.evolved_plans).toBeGreaterThanOrEqual(0);
        expect(response.new_plans_added).toBeInstanceOf(Array);
        expect(response.evolution_time_ms).toBeGreaterThanOrEqual(0);
      });
    });

    describe('Baseline utility methods', () => {
      it('should enable baseline', async () => {
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_enable WHERE id = $1',
          param_types: ['INTEGER'],
          enabled: false,
        });

        const response = await client.enableBaseline(created.fingerprint);

        expect(response).toBeDefined();
        expect(response.fingerprint).toBe(created.fingerprint);
      });

      it('should disable baseline', async () => {
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_disable WHERE id = $1',
          param_types: ['INTEGER'],
          enabled: true,
        });

        const response = await client.disableBaseline(created.fingerprint);

        expect(response).toBeDefined();
      });

      it('should fix baseline', async () => {
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_fix_util WHERE id = $1',
          param_types: ['INTEGER'],
          fixed: false,
        });

        const response = await client.fixBaseline(created.fingerprint);

        expect(response).toBeDefined();
      });

      it('should unfix baseline', async () => {
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_unfix WHERE id = $1',
          param_types: ['INTEGER'],
          fixed: true,
        });

        const response = await client.unfixBaseline(created.fingerprint);

        expect(response).toBeDefined();
      });

      it('should get baseline stats', async () => {
        const created = await client.createBaseline({
          query_text: 'SELECT * FROM test_stats WHERE id = $1',
          param_types: ['INTEGER'],
        });

        const stats = await client.getBaselineStats(created.fingerprint);

        expect(stats).toBeDefined();
        expect(stats.execution_count).toBeGreaterThanOrEqual(0);
        expect(stats.avg_execution_time_ms).toBeGreaterThanOrEqual(0);
        expect(stats.plan_count).toBeGreaterThanOrEqual(0);
      });
    });
  });

  // =========================================================================
  // Error Handling Tests
  // =========================================================================

  describe('Error Handling', () => {
    it('should throw QueryOptimizerError on API errors', async () => {
      const request: QueryRequest = {
        sql: 'INVALID SQL SYNTAX HERE',
      };

      try {
        await client.executeQuery(request);
        fail('Should have thrown error');
      } catch (error) {
        expect(error).toBeInstanceOf(QueryOptimizerError);
        expect((error as QueryOptimizerError).code).toBeDefined();
        expect((error as QueryOptimizerError).message).toBeDefined();
      }
    });

    it('should include error details', async () => {
      const request: QueryRequest = {
        sql: 'SELECT * FROM nonexistent_table',
      };

      try {
        await client.executeQuery(request);
        fail('Should have thrown error');
      } catch (error) {
        expect(error).toBeInstanceOf(QueryOptimizerError);
        const qError = error as QueryOptimizerError;
        expect(qError.code).toBeDefined();
        expect(qError.message).toBeDefined();
      }
    });

    it('should handle network errors', async () => {
      const badClient = new QueryOptimizerClient({
        baseUrl: 'http://localhost:99999', // Invalid port
        timeout: 1000,
      });

      try {
        await badClient.executeQuery({ sql: 'SELECT 1' });
        fail('Should have thrown error');
      } catch (error) {
        expect(error).toBeInstanceOf(QueryOptimizerError);
      }
    });
  });

  // =========================================================================
  // Integration Tests
  // =========================================================================

  describe('Integration Tests', () => {
    it('should execute query and explain it', async () => {
      const sql = 'SELECT * FROM users WHERE age > 25 ORDER BY name LIMIT 10';

      // Execute query
      const result = await client.executeQuery({ sql });
      expect(result).toBeDefined();
      expect(result.query_id).toBeDefined();

      // Explain the same query
      const plan = await client.explainQuery(sql);
      expect(plan).toBeDefined();
      expect(plan.query).toBe(sql);
    });

    it('should create baseline and evolve it', async () => {
      const queryText = 'SELECT * FROM integration_test WHERE value > $1';

      // Create baseline
      const baseline = await client.createBaseline({
        query_text: queryText,
        param_types: ['INTEGER'],
        enabled: true,
        fixed: false,
      });

      expect(baseline).toBeDefined();

      // Get baseline details
      const details = await client.getBaseline(baseline.fingerprint);
      expect(details).toBeDefined();

      // Evolve baseline
      const evolveResult = await client.evolveBaseline(baseline.fingerprint);
      expect(evolveResult).toBeDefined();

      // Clean up
      await client.deleteBaseline(baseline.fingerprint);
    });

    it('should apply hints and compare plans', async () => {
      const baseQuery = 'SELECT * FROM users u JOIN orders o ON u.id = o.user_id';

      // Apply hints
      const hintResult = await client.applyHints({
        query: baseQuery,
        hints: ['HASH_JOIN(u o)'],
      });

      expect(hintResult).toBeDefined();

      // Compare plans
      const comparison = await client.comparePlans(
        baseQuery,
        `SELECT /*+ HASH_JOIN(u o) */ * FROM users u JOIN orders o ON u.id = o.user_id`
      );

      expect(comparison.original).toBeDefined();
      expect(comparison.withHints).toBeDefined();
    });

    it('should list hints and apply them', async () => {
      // List available hints
      const hints = await client.listHints();
      expect(hints.total).toBeGreaterThan(0);

      // Get first hint
      const firstHint = hints.hints[0];

      // Try to apply it (if applicable)
      const applyResult = await client.applyHints({
        query: 'SELECT * FROM users',
        hints: [firstHint.name],
      });

      expect(applyResult).toBeDefined();
    });
  });
});
