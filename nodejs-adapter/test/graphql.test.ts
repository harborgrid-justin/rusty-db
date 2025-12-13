/**
 * RustyDB GraphQL Client Tests
 *
 * Comprehensive test suite for the GraphQL client
 */

import { RustyDBGraphQLClient, createGraphQLClient } from '../src/api/graphql-client';
import {
  DataType,
  SortOrder,
  FilterOp,
  AggregateFunc,
  JoinType,
  IsolationLevel,
  ChangeType,
  ConstraintTypeEnum,
  StringFunctionTypeEnum,
  ParameterMode,
} from '../src/types/graphql-types';

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

const TEST_CONFIG = {
  endpoint: 'http://localhost:5432/graphql',
  wsEndpoint: 'ws://localhost:5432/graphql',
  headers: {
    'Authorization': 'Bearer test-token',
  },
  timeout: 10000,
};

// ============================================================================
// QUERY OPERATION TESTS
// ============================================================================

describe('GraphQL Query Operations', () => {
  let client: RustyDBGraphQLClient;

  beforeAll(() => {
    client = createGraphQLClient(TEST_CONFIG);
  });

  afterAll(async () => {
    await client.close();
  });

  describe('Schema Queries', () => {
    test('should get all schemas', async () => {
      const schemas = await client.getSchemas();
      expect(Array.isArray(schemas)).toBe(true);
    });

    test('should get specific schema', async () => {
      const schema = await client.getSchema('public');
      if (schema) {
        expect(schema.name).toBe('public');
      }
    });
  });

  describe('Table Queries', () => {
    test('should get all tables', async () => {
      const tables = await client.getTables();
      expect(Array.isArray(tables)).toBe(true);
    });

    test('should get tables with filters', async () => {
      const tables = await client.getTables({
        schema: 'public',
        limit: 10,
        offset: 0,
      });
      expect(Array.isArray(tables)).toBe(true);
    });

    test('should get specific table', async () => {
      const table = await client.getTable('users', 'public');
      if (table) {
        expect(table.name).toBe('users');
        expect(table.schema).toBe('public');
      }
    });
  });

  describe('Data Queries', () => {
    test('should query table with basic filter', async () => {
      const result = await client.queryTable({
        table: 'users',
        where: {
          condition: {
            field: 'status',
            op: FilterOp.Eq,
            value: 'active',
          },
        },
        limit: 10,
      });

      if (result.__typename === 'QuerySuccess') {
        expect(Array.isArray(result.rows)).toBe(true);
        expect(typeof result.totalCount).toBe('string');
      }
    });

    test('should query table with complex filter', async () => {
      const result = await client.queryTable({
        table: 'orders',
        where: {
          and: [
            {
              condition: {
                field: 'status',
                op: FilterOp.Eq,
                value: 'pending',
              },
            },
            {
              condition: {
                field: 'total',
                op: FilterOp.Gt,
                value: 100,
              },
            },
          ],
        },
        orderBy: [
          { field: 'created_at', order: SortOrder.Desc },
        ],
        limit: 20,
        offset: 0,
      });

      if (result.__typename === 'QuerySuccess') {
        expect(result.rows).toBeDefined();
      }
    });

    test('should query multiple tables with joins', async () => {
      const result = await client.queryTables({
        tables: ['users', 'orders'],
        joins: [
          {
            table: 'orders',
            joinType: JoinType.Left,
            onField: 'user_id',
            otherField: 'id',
          },
        ],
        limit: 10,
      });

      expect(result).toBeDefined();
    });

    test('should query with cursor pagination', async () => {
      const result = await client.queryTableConnection({
        table: 'users',
        first: 10,
      });

      expect(result.edges).toBeDefined();
      expect(result.pageInfo).toBeDefined();
      expect(typeof result.totalCount).toBe('string');
    });

    test('should get single row by ID', async () => {
      const row = await client.getRow('users', '123');
      if (row) {
        expect(row.id).toBe('123');
        expect(row.tableName).toBe('users');
      }
    });
  });

  describe('Aggregation Queries', () => {
    test('should perform basic aggregation', async () => {
      const results = await client.aggregate({
        table: 'orders',
        aggregates: [
          {
            function: AggregateFunc.Count,
            field: 'id',
            alias: 'total_orders',
          },
          {
            function: AggregateFunc.Sum,
            field: 'total',
            alias: 'total_revenue',
          },
          {
            function: AggregateFunc.Avg,
            field: 'total',
            alias: 'avg_order_value',
          },
        ],
      });

      expect(Array.isArray(results)).toBe(true);
      expect(results.length).toBe(3);
    });

    test('should perform aggregation with grouping', async () => {
      const results = await client.aggregate({
        table: 'orders',
        aggregates: [
          {
            function: AggregateFunc.Count,
            field: 'id',
          },
          {
            function: AggregateFunc.Sum,
            field: 'total',
          },
        ],
        groupBy: ['status'],
      });

      expect(Array.isArray(results)).toBe(true);
    });

    test('should count rows', async () => {
      const count = await client.count('users');
      expect(typeof count).toBe('string');
    });

    test('should count rows with filter', async () => {
      const count = await client.count('users', {
        condition: {
          field: 'status',
          op: FilterOp.Eq,
          value: 'active',
        },
      });
      expect(typeof count).toBe('string');
    });
  });

  describe('Advanced Queries', () => {
    test('should execute raw SQL', async () => {
      const result = await client.executeSql(
        'SELECT * FROM users WHERE status = $1 LIMIT 10',
        ['active']
      );

      expect(result).toBeDefined();
    });

    test('should perform full-text search', async () => {
      const result = await client.search({
        query: 'john smith',
        tables: ['users', 'customers'],
        fields: ['name', 'email'],
        limit: 10,
      });

      expect(result.results).toBeDefined();
      expect(typeof result.totalCount).toBe('string');
    });

    test('should get query execution plan', async () => {
      const plan = await client.explain({
        table: 'users',
        where: {
          condition: {
            field: 'email',
            op: FilterOp.Eq,
            value: 'test@example.com',
          },
        },
      });

      expect(plan.planText).toBeDefined();
      expect(plan.estimatedCost).toBeGreaterThanOrEqual(0);
    });

    test('should execute UNION query', async () => {
      const result = await client.executeUnion([
        'SELECT id, name FROM users WHERE status = \'active\'',
        'SELECT id, name FROM users WHERE status = \'pending\'',
      ], false);

      expect(result).toBeDefined();
    });
  });
});

// ============================================================================
// MUTATION OPERATION TESTS
// ============================================================================

describe('GraphQL Mutation Operations', () => {
  let client: RustyDBGraphQLClient;

  beforeAll(() => {
    client = createGraphQLClient(TEST_CONFIG);
  });

  afterAll(async () => {
    await client.close();
  });

  describe('Data Manipulation', () => {
    test('should insert one row', async () => {
      const result = await client.insertOne('users', {
        name: 'John Doe',
        email: 'john@example.com',
        status: 'active',
      });

      if (result.__typename === 'MutationSuccess') {
        expect(result.affectedRows).toBe(1);
        expect(result.returning).toBeDefined();
      }
    });

    test('should insert many rows', async () => {
      const result = await client.insertMany('users', [
        { name: 'Alice', email: 'alice@example.com' },
        { name: 'Bob', email: 'bob@example.com' },
        { name: 'Charlie', email: 'charlie@example.com' },
      ]);

      if (result.__typename === 'MutationSuccess') {
        expect(result.affectedRows).toBe(3);
      }
    });

    test('should update one row', async () => {
      const result = await client.updateOne('users', '123', {
        name: 'Jane Doe',
        status: 'inactive',
      });

      if (result.__typename === 'MutationSuccess') {
        expect(result.affectedRows).toBe(1);
      }
    });

    test('should update many rows', async () => {
      const result = await client.updateMany(
        'users',
        {
          condition: {
            field: 'status',
            op: FilterOp.Eq,
            value: 'pending',
          },
        },
        { status: 'active' }
      );

      if (result.__typename === 'MutationSuccess') {
        expect(result.affectedRows).toBeGreaterThanOrEqual(0);
      }
    });

    test('should delete one row', async () => {
      const result = await client.deleteOne('users', '123');

      if (result.__typename === 'MutationSuccess') {
        expect(result.affectedRows).toBe(1);
      }
    });

    test('should delete many rows', async () => {
      const result = await client.deleteMany('users', {
        condition: {
          field: 'status',
          op: FilterOp.Eq,
          value: 'deleted',
        },
      });

      expect(result).toBeDefined();
    });

    test('should upsert row', async () => {
      const result = await client.upsert(
        'users',
        ['email'],
        {
          email: 'john@example.com',
          name: 'John Doe Updated',
          status: 'active',
        }
      );

      if (result.__typename === 'MutationSuccess') {
        expect(result.affectedRows).toBe(1);
      }
    });

    test('should bulk insert', async () => {
      const data = Array.from({ length: 100 }, (_, i) => ({
        name: `User ${i}`,
        email: `user${i}@example.com`,
      }));

      const result = await client.bulkInsert('users', data, 50);

      if (result.__typename === 'MutationSuccess') {
        expect(result.affectedRows).toBe(100);
      }
    });
  });

  describe('Transaction Operations', () => {
    test('should begin transaction', async () => {
      const result = await client.beginTransaction(IsolationLevel.ReadCommitted);

      expect(result.transactionId).toBeDefined();
      expect(result.status).toBeDefined();
    });

    test('should commit transaction', async () => {
      const tx = await client.beginTransaction();
      const result = await client.commitTransaction(tx.transactionId);

      expect(result.status).toBeDefined();
    });

    test('should rollback transaction', async () => {
      const tx = await client.beginTransaction();
      const result = await client.rollbackTransaction(tx.transactionId);

      expect(result.status).toBeDefined();
    });

    test('should execute transaction', async () => {
      const result = await client.executeTransaction(
        [
          {
            operationType: 'INSERT',
            table: 'users',
            data: { name: 'Test User', email: 'test@example.com' },
          },
          {
            operationType: 'UPDATE',
            table: 'users',
            whereClause: {
              condition: {
                field: 'id',
                op: FilterOp.Eq,
                value: '123',
              },
            },
            data: { status: 'active' },
          },
        ],
        IsolationLevel.Serializable
      );

      expect(result.success).toBeDefined();
      expect(Array.isArray(result.results)).toBe(true);
    });
  });

  describe('DDL Operations', () => {
    test('should create database', async () => {
      const result = await client.createDatabase('test_db', true);

      if (result.__typename === 'DdlSuccess') {
        expect(result.success).toBe(true);
      }
    });

    test('should drop database', async () => {
      const result = await client.dropDatabase('test_db', true);

      expect(result).toBeDefined();
    });

    test('should backup database', async () => {
      const result = await client.backupDatabase('main_db', '/backups/main_db.bak', true);

      expect(result).toBeDefined();
    });

    test('should add column to table', async () => {
      const result = await client.alterTableAddColumn('users', {
        name: 'middle_name',
        dataType: 'VARCHAR',
        nullable: true,
      });

      expect(result).toBeDefined();
    });

    test('should drop column from table', async () => {
      const result = await client.alterTableDropColumn('users', 'middle_name', true);

      expect(result).toBeDefined();
    });

    test('should modify column', async () => {
      const result = await client.alterTableModifyColumn('users', {
        name: 'email',
        dataType: 'VARCHAR',
        nullable: false,
        unique: true,
      });

      expect(result).toBeDefined();
    });

    test('should add constraint', async () => {
      const result = await client.alterTableAddConstraint('users', {
        name: 'users_email_unique',
        constraintType: ConstraintTypeEnum.Unique,
        columns: ['email'],
      });

      expect(result).toBeDefined();
    });

    test('should drop constraint', async () => {
      const result = await client.alterTableDropConstraint('users', 'users_email_unique', true);

      expect(result).toBeDefined();
    });

    test('should truncate table', async () => {
      const result = await client.truncateTable('temp_table');

      expect(result).toBeDefined();
    });
  });

  describe('View Operations', () => {
    test('should create view', async () => {
      const result = await client.createView(
        'active_users',
        'SELECT * FROM users WHERE status = \'active\'',
        true
      );

      expect(result).toBeDefined();
    });

    test('should drop view', async () => {
      const result = await client.dropView('active_users', true);

      expect(result).toBeDefined();
    });
  });

  describe('Index Operations', () => {
    test('should create index', async () => {
      const result = await client.createIndex({
        table: 'users',
        indexName: 'idx_users_email',
        columns: ['email'],
        unique: true,
        ifNotExists: true,
      });

      expect(result).toBeDefined();
    });

    test('should drop index', async () => {
      const result = await client.dropIndex('idx_users_email', 'users', true);

      expect(result).toBeDefined();
    });
  });

  describe('Stored Procedure Operations', () => {
    test('should create stored procedure', async () => {
      const result = await client.createProcedure(
        'get_user_stats',
        [
          { name: 'user_id', dataType: 'INTEGER', mode: ParameterMode.In },
          { name: 'result', dataType: 'JSON', mode: ParameterMode.Out },
        ],
        'BEGIN SELECT * FROM users WHERE id = user_id; END;',
        true
      );

      expect(result).toBeDefined();
    });

    test('should execute stored procedure', async () => {
      const result = await client.executeProcedure('get_user_stats', [123]);

      expect(result).toBeDefined();
    });
  });

  describe('Advanced Operations', () => {
    test('should insert into select', async () => {
      const result = await client.insertIntoSelect(
        'users_backup',
        'SELECT * FROM users WHERE created_at < NOW() - INTERVAL \'1 year\'',
        ['id', 'name', 'email']
      );

      expect(result).toBeDefined();
    });

    test('should select into', async () => {
      const result = await client.selectInto(
        'active_users_snapshot',
        'SELECT * FROM users WHERE status = \'active\''
      );

      expect(result).toBeDefined();
    });
  });

  describe('String Functions', () => {
    test('should execute string function', async () => {
      const result = await client.executeStringFunction(
        StringFunctionTypeEnum.Upper,
        ['hello world']
      );

      expect(result.result).toBe('HELLO WORLD');
    });

    test('should batch execute string functions', async () => {
      const result = await client.batchStringFunctions([
        {
          functionType: StringFunctionTypeEnum.Upper,
          parameters: ['hello'],
        },
        {
          functionType: StringFunctionTypeEnum.Lower,
          parameters: ['WORLD'],
        },
        {
          functionType: StringFunctionTypeEnum.Concat,
          parameters: ['Hello', ' ', 'World'],
        },
      ]);

      expect(Array.isArray(result.results)).toBe(true);
      expect(result.results.length).toBe(3);
    });
  });
});

// ============================================================================
// SUBSCRIPTION OPERATION TESTS
// ============================================================================

describe('GraphQL Subscription Operations', () => {
  let client: RustyDBGraphQLClient;

  beforeAll(() => {
    client = createGraphQLClient(TEST_CONFIG);
  });

  afterAll(async () => {
    await client.close();
  });

  describe('Table Change Subscriptions', () => {
    test('should subscribe to table changes', (done) => {
      const unsubscribe = client.subscribeTableChanges(
        'users',
        undefined,
        (data) => {
          expect(data.table).toBe('users');
          expect(data.changeType).toBeDefined();
          unsubscribe();
          done();
        }
      );

      expect(unsubscribe).toBeInstanceOf(Function);
    });

    test('should subscribe to row insertions', (done) => {
      const unsubscribe = client.subscribeRowInserted(
        'orders',
        undefined,
        (data) => {
          expect(data.table).toBe('orders');
          expect(data.row).toBeDefined();
          unsubscribe();
          done();
        }
      );

      expect(unsubscribe).toBeInstanceOf(Function);
    });

    test('should subscribe to row updates', (done) => {
      const unsubscribe = client.subscribeRowUpdated(
        'users',
        {
          condition: {
            field: 'status',
            op: FilterOp.Eq,
            value: 'active',
          },
        },
        (data) => {
          expect(data.oldRow).toBeDefined();
          expect(data.newRow).toBeDefined();
          expect(data.changedFields).toBeDefined();
          unsubscribe();
          done();
        }
      );

      expect(unsubscribe).toBeInstanceOf(Function);
    });

    test('should subscribe to row deletions', (done) => {
      const unsubscribe = client.subscribeRowDeleted(
        'temp_data',
        undefined,
        (data) => {
          expect(data.table).toBe('temp_data');
          expect(data.id).toBeDefined();
          unsubscribe();
          done();
        }
      );

      expect(unsubscribe).toBeInstanceOf(Function);
    });

    test('should subscribe to specific row changes', (done) => {
      const unsubscribe = client.subscribeRowChanges(
        'users',
        '123',
        (data) => {
          expect(data.id).toBe('123');
          expect(data.changeType).toBeDefined();
          unsubscribe();
          done();
        }
      );

      expect(unsubscribe).toBeInstanceOf(Function);
    });
  });

  describe('Aggregate Subscriptions', () => {
    test('should subscribe to aggregate changes', (done) => {
      const unsubscribe = client.subscribeAggregateChanges(
        'orders',
        [
          { function: AggregateFunc.Count, field: 'id' },
          { function: AggregateFunc.Sum, field: 'total' },
        ],
        undefined,
        5,
        (data) => {
          expect(data.results).toBeDefined();
          expect(Array.isArray(data.results)).toBe(true);
          unsubscribe();
          done();
        }
      );

      expect(unsubscribe).toBeInstanceOf(Function);
    });
  });

  describe('Query Subscriptions', () => {
    test('should subscribe to query changes', (done) => {
      const unsubscribe = client.subscribeQueryChanges(
        'users',
        {
          condition: {
            field: 'status',
            op: FilterOp.Eq,
            value: 'active',
          },
        },
        [{ field: 'created_at', order: SortOrder.Desc }],
        10,
        5,
        (data) => {
          expect(data.rows).toBeDefined();
          expect(typeof data.totalCount).toBe('string');
          unsubscribe();
          done();
        }
      );

      expect(unsubscribe).toBeInstanceOf(Function);
    });
  });

  describe('Heartbeat Subscription', () => {
    test('should subscribe to heartbeat', (done) => {
      const unsubscribe = client.subscribeHeartbeat(
        30,
        (data) => {
          expect(typeof data.sequence).toBe('number');
          expect(data.timestamp).toBeDefined();
          unsubscribe();
          done();
        }
      );

      expect(unsubscribe).toBeInstanceOf(Function);
    });
  });
});

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

describe('Error Handling', () => {
  let client: RustyDBGraphQLClient;

  beforeAll(() => {
    client = createGraphQLClient({
      endpoint: 'http://localhost:9999/graphql', // Invalid endpoint
      timeout: 1000,
    });
  });

  test('should handle connection errors', async () => {
    await expect(client.getSchemas()).rejects.toThrow();
  });

  test('should handle timeout errors', async () => {
    const slowClient = createGraphQLClient({
      endpoint: TEST_CONFIG.endpoint,
      timeout: 1, // Very short timeout
    });

    await expect(slowClient.getSchemas()).rejects.toThrow(/timeout/i);
  });

  test('should handle GraphQL errors', async () => {
    const result = await client.queryTable({
      table: 'non_existent_table',
    });

    if (result.__typename === 'QueryError') {
      expect(result.message).toBeDefined();
      expect(result.code).toBeDefined();
    }
  });
});

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

describe('Integration Tests', () => {
  let client: RustyDBGraphQLClient;

  beforeAll(() => {
    client = createGraphQLClient(TEST_CONFIG);
  });

  afterAll(async () => {
    await client.close();
  });

  test('should perform complete CRUD workflow', async () => {
    // Create
    const insertResult = await client.insertOne('users', {
      name: 'Integration Test User',
      email: 'integration@test.com',
      status: 'active',
    });

    let userId: string | undefined;
    if (insertResult.__typename === 'MutationSuccess' && insertResult.returning) {
      userId = insertResult.returning[0].id;
      expect(userId).toBeDefined();
    }

    if (!userId) return;

    // Read
    const row = await client.getRow('users', userId);
    expect(row?.id).toBe(userId);

    // Update
    const updateResult = await client.updateOne('users', userId, {
      name: 'Updated User',
    });

    if (updateResult.__typename === 'MutationSuccess') {
      expect(updateResult.affectedRows).toBe(1);
    }

    // Delete
    const deleteResult = await client.deleteOne('users', userId);

    if (deleteResult.__typename === 'MutationSuccess') {
      expect(deleteResult.affectedRows).toBe(1);
    }
  });

  test('should perform transaction workflow', async () => {
    const result = await client.executeTransaction([
      {
        operationType: 'INSERT',
        table: 'users',
        data: { name: 'TX User 1', email: 'tx1@test.com' },
      },
      {
        operationType: 'INSERT',
        table: 'users',
        data: { name: 'TX User 2', email: 'tx2@test.com' },
      },
      {
        operationType: 'INSERT',
        table: 'orders',
        data: { user_id: 1, total: 100 },
      },
    ]);

    expect(result.success).toBeDefined();
  });
});

// Export for use in other test files
export { TEST_CONFIG };
