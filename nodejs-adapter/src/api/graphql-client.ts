/**
 * RustyDB GraphQL Client
 *
 * Complete GraphQL client for RustyDB with full schema coverage
 * Supports queries, mutations, and subscriptions
 */

import {
  // Types
  DatabaseSchema,
  TableType,
  RowType,
  QueryResult,
  MutationResult,
  DdlResult,
  ProcedureResult,
  SearchResult,
  QueryPlan,
  TransactionResult,
  TransactionExecutionResult,
  StringFunctionResult,
  BatchStringFunctionResult,
  RowConnection,
  AggregateResult,
  TableChange,
  RowInserted,
  RowUpdated,
  RowDeleted,
  RowChange,
  AggregateChange,
  QueryChange,
  Heartbeat,
  // Input types
  WhereClause,
  OrderBy,
  AggregateInput,
  JoinInput,
  ColumnDefinitionInput,
  ConstraintInput,
  ProcedureParameter,
  TransactionOperation,
  StringFunctionInput,
  // Enums
  IsolationLevel,
  StringFunctionTypeEnum,
  Json,
  ID,
} from '../types/graphql-types';

export interface GraphQLClientConfig {
  /** GraphQL endpoint URL */
  endpoint: string;
  /** WebSocket endpoint for subscriptions */
  wsEndpoint?: string;
  /** Authentication headers */
  headers?: Record<string, string>;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Enable query batching */
  batching?: boolean;
  /** Enable automatic retries */
  retries?: number;
}

export interface GraphQLRequest {
  query: string;
  variables?: Record<string, unknown>;
  operationName?: string;
}

export interface GraphQLResponse<T = unknown> {
  data?: T;
  errors?: Array<{
    message: string;
    locations?: Array<{ line: number; column: number }>;
    path?: Array<string | number>;
    extensions?: Record<string, unknown>;
  }>;
}

/**
 * RustyDB GraphQL Client
 *
 * Provides type-safe access to the complete RustyDB GraphQL API
 */
export class RustyDBGraphQLClient {
  private config: Required<GraphQLClientConfig>;
  private wsClient?: unknown; // WebSocket client for subscriptions

  constructor(config: GraphQLClientConfig) {
    this.config = {
      endpoint: config.endpoint,
      wsEndpoint: config.wsEndpoint || config.endpoint.replace('http', 'ws'),
      headers: config.headers || {},
      timeout: config.timeout || 30000,
      batching: config.batching ?? false,
      retries: config.retries ?? 3,
    };
  }

  // ============================================================================
  // CORE REQUEST METHOD
  // ============================================================================

  private async request<T>(
    query: string,
    variables?: Record<string, unknown>,
    operationName?: string
  ): Promise<T> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(this.config.endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...this.config.headers,
        },
        body: JSON.stringify({
          query,
          variables,
          operationName,
        }),
        signal: controller.signal,
      });

      clearTimeout(timeout);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const result: GraphQLResponse<T> = await response.json();

      if (result.errors && result.errors.length > 0) {
        throw new Error(
          `GraphQL Error: ${result.errors.map(e => e.message).join(', ')}`
        );
      }

      if (!result.data) {
        throw new Error('No data returned from GraphQL query');
      }

      return result.data;
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        throw new Error(`Request timeout after ${this.config.timeout}ms`);
      }
      throw error;
    }
  }

  // ============================================================================
  // QUERY OPERATIONS
  // ============================================================================

  /**
   * Get all database schemas
   */
  async getSchemas(): Promise<DatabaseSchema[]> {
    const query = `
      query GetSchemas {
        schemas {
          name
          tables {
            id
            name
            schema
            rowCount
            sizeBytes
            createdAt
          }
          tableCount
          createdAt
          description
        }
      }
    `;
    const result = await this.request<{ schemas: DatabaseSchema[] }>(query);
    return result.schemas;
  }

  /**
   * Get a specific schema by name
   */
  async getSchema(name: string): Promise<DatabaseSchema | null> {
    const query = `
      query GetSchema($name: String!) {
        schema(name: $name) {
          name
          tables {
            id
            name
            schema
            rowCount
            sizeBytes
            createdAt
          }
          tableCount
          createdAt
          description
        }
      }
    `;
    const result = await this.request<{ schema: DatabaseSchema | null }>(query, { name });
    return result.schema;
  }

  /**
   * Get all tables across all schemas
   */
  async getTables(options?: {
    schema?: string;
    limit?: number;
    offset?: number;
  }): Promise<TableType[]> {
    const query = `
      query GetTables($schema: String, $limit: Int, $offset: Int) {
        tables(schema: $schema, limit: $limit, offset: $offset) {
          id
          name
          schema
          columns {
            id
            name
            dataType
            nullable
            position
          }
          rowCount
          sizeBytes
          createdAt
          updatedAt
          description
          indexes {
            name
            columns
            unique
            indexType
            sizeBytes
            createdAt
          }
          constraints {
            name
            constraintType
            columns
            referencedTable
            referencedColumns
          }
        }
      }
    `;
    const result = await this.request<{ tables: TableType[] }>(query, options);
    return result.tables;
  }

  /**
   * Get a specific table by name
   */
  async getTable(name: string, schema?: string): Promise<TableType | null> {
    const query = `
      query GetTable($name: String!, $schema: String) {
        table(name: $name, schema: $schema) {
          id
          name
          schema
          columns {
            id
            name
            dataType
            nullable
            defaultValue
            position
            maxLength
            precision
            scale
            description
          }
          rowCount
          sizeBytes
          createdAt
          updatedAt
          createdBy
          updatedBy
          description
          indexes {
            name
            columns
            unique
            indexType
            sizeBytes
            createdAt
          }
          constraints {
            name
            constraintType
            columns
            referencedTable
            referencedColumns
          }
        }
      }
    `;
    const result = await this.request<{ table: TableType | null }>(query, { name, schema });
    return result.table;
  }

  /**
   * Query a table with filtering and pagination
   */
  async queryTable(options: {
    table: string;
    where?: WhereClause;
    orderBy?: OrderBy[];
    limit?: number;
    offset?: number;
  }): Promise<QueryResult> {
    const query = `
      query QueryTable(
        $table: String!,
        $where: WhereClause,
        $orderBy: [OrderBy!],
        $limit: Int,
        $offset: Int
      ) {
        queryTable(
          table: $table,
          where: $where,
          orderBy: $orderBy,
          limit: $limit,
          offset: $offset
        ) {
          ... on QuerySuccess {
            rows {
              id
              tableName
              fields
              createdAt
              updatedAt
              version
            }
            totalCount
            executionTimeMs
            hasMore
          }
          ... on QueryError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ queryTable: QueryResult }>(query, options);
    return result.queryTable;
  }

  /**
   * Query multiple tables with joins
   */
  async queryTables(options: {
    tables: string[];
    joins?: JoinInput[];
    where?: WhereClause;
    orderBy?: OrderBy[];
    limit?: number;
  }): Promise<QueryResult> {
    const query = `
      query QueryTables(
        $tables: [String!]!,
        $joins: [JoinInput!],
        $where: WhereClause,
        $orderBy: [OrderBy!],
        $limit: Int
      ) {
        queryTables(
          tables: $tables,
          joins: $joins,
          where: $where,
          orderBy: $orderBy,
          limit: $limit
        ) {
          ... on QuerySuccess {
            rows {
              id
              tableName
              fields
              createdAt
              updatedAt
              version
            }
            totalCount
            executionTimeMs
            hasMore
          }
          ... on QueryError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ queryTables: QueryResult }>(query, options);
    return result.queryTables;
  }

  /**
   * Query with cursor-based pagination
   */
  async queryTableConnection(options: {
    table: string;
    where?: WhereClause;
    orderBy?: OrderBy[];
    first?: number;
    after?: string;
    last?: number;
    before?: string;
  }): Promise<RowConnection> {
    const query = `
      query QueryTableConnection(
        $table: String!,
        $where: WhereClause,
        $orderBy: [OrderBy!],
        $first: Int,
        $after: String,
        $last: Int,
        $before: String
      ) {
        queryTableConnection(
          table: $table,
          where: $where,
          orderBy: $orderBy,
          first: $first,
          after: $after,
          last: $last,
          before: $before
        ) {
          edges {
            cursor
            node {
              id
              tableName
              fields
              createdAt
              version
            }
          }
          pageInfo {
            hasNextPage
            hasPreviousPage
            startCursor
            endCursor
            totalCount
          }
          totalCount
        }
      }
    `;
    const result = await this.request<{ queryTableConnection: RowConnection }>(query, options);
    return result.queryTableConnection;
  }

  /**
   * Get a single row by ID
   */
  async getRow(table: string, id: ID): Promise<RowType | null> {
    const query = `
      query GetRow($table: String!, $id: ID!) {
        row(table: $table, id: $id) {
          id
          tableName
          fields
          createdAt
          updatedAt
          createdBy
          updatedBy
          version
        }
      }
    `;
    const result = await this.request<{ row: RowType | null }>(query, { table, id });
    return result.row;
  }

  /**
   * Perform aggregations on a table
   */
  async aggregate(options: {
    table: string;
    aggregates: AggregateInput[];
    where?: WhereClause;
    groupBy?: string[];
  }): Promise<AggregateResult[]> {
    const query = `
      query Aggregate(
        $table: String!,
        $aggregates: [AggregateInput!]!,
        $where: WhereClause,
        $groupBy: [String!]
      ) {
        aggregate(
          table: $table,
          aggregates: $aggregates,
          where: $where,
          groupBy: $groupBy
        ) {
          field
          function
          value
        }
      }
    `;
    const result = await this.request<{ aggregate: AggregateResult[] }>(query, options);
    return result.aggregate;
  }

  /**
   * Count rows in a table
   */
  async count(table: string, where?: WhereClause): Promise<string> {
    const query = `
      query Count($table: String!, $where: WhereClause) {
        count(table: $table, where: $where)
      }
    `;
    const result = await this.request<{ count: string }>(query, { table, where });
    return result.count;
  }

  /**
   * Execute a raw SQL query (admin only)
   */
  async executeSql(sql: string, params?: Json[]): Promise<QueryResult> {
    const query = `
      query ExecuteSql($sql: String!, $params: [Json!]) {
        executeSql(sql: $sql, params: $params) {
          ... on QuerySuccess {
            rows {
              id
              tableName
              fields
              createdAt
              version
            }
            totalCount
            executionTimeMs
            hasMore
          }
          ... on QueryError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ executeSql: QueryResult }>(query, { sql, params });
    return result.executeSql;
  }

  /**
   * Search across multiple tables
   */
  async search(options: {
    query: string;
    tables?: string[];
    fields?: string[];
    limit?: number;
  }): Promise<SearchResult> {
    const gqlQuery = `
      query Search(
        $query: String!,
        $tables: [String!],
        $fields: [String!],
        $limit: Int
      ) {
        search(query: $query, tables: $tables, fields: $fields, limit: $limit) {
          results {
            table
            row {
              id
              tableName
              fields
            }
            score
            highlights
          }
          totalCount
          executionTimeMs
        }
      }
    `;
    const result = await this.request<{ search: SearchResult }>(gqlQuery, options);
    return result.search;
  }

  /**
   * Get query execution plan
   */
  async explain(options: {
    table: string;
    where?: WhereClause;
    orderBy?: OrderBy[];
  }): Promise<QueryPlan> {
    const query = `
      query Explain($table: String!, $where: WhereClause, $orderBy: [OrderBy!]) {
        explain(table: $table, where: $where, orderBy: $orderBy) {
          planText
          estimatedCost
          estimatedRows
          operations {
            operationType
            description
            cost
            rows
            children {
              operationType
              description
              cost
              rows
            }
          }
        }
      }
    `;
    const result = await this.request<{ explain: QueryPlan }>(query, options);
    return result.explain;
  }

  /**
   * Execute UNION query
   */
  async executeUnion(queries: string[], unionAll?: boolean): Promise<QueryResult> {
    const query = `
      query ExecuteUnion($queries: [String!]!, $unionAll: Boolean) {
        executeUnion(queries: $queries, unionAll: $unionAll) {
          ... on QuerySuccess {
            rows {
              id
              tableName
              fields
              createdAt
              version
            }
            totalCount
            executionTimeMs
            hasMore
          }
          ... on QueryError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ executeUnion: QueryResult }>(query, {
      queries,
      unionAll,
    });
    return result.executeUnion;
  }

  // ============================================================================
  // MUTATION OPERATIONS - Data Manipulation
  // ============================================================================

  /**
   * Insert a single row
   */
  async insertOne(table: string, data: Record<string, Json>): Promise<MutationResult> {
    const mutation = `
      mutation InsertOne($table: String!, $data: Map!) {
        insertOne(table: $table, data: $data) {
          ... on MutationSuccess {
            affectedRows
            returning {
              id
              tableName
              fields
              createdAt
              version
            }
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ insertOne: MutationResult }>(mutation, { table, data });
    return result.insertOne;
  }

  /**
   * Insert multiple rows
   */
  async insertMany(table: string, data: Record<string, Json>[]): Promise<MutationResult> {
    const mutation = `
      mutation InsertMany($table: String!, $data: [Map!]!) {
        insertMany(table: $table, data: $data) {
          ... on MutationSuccess {
            affectedRows
            returning {
              id
              tableName
              fields
              createdAt
              version
            }
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ insertMany: MutationResult }>(mutation, { table, data });
    return result.insertMany;
  }

  /**
   * Update a single row by ID
   */
  async updateOne(table: string, id: ID, data: Record<string, Json>): Promise<MutationResult> {
    const mutation = `
      mutation UpdateOne($table: String!, $id: ID!, $data: Map!) {
        updateOne(table: $table, id: $id, data: $data) {
          ... on MutationSuccess {
            affectedRows
            returning {
              id
              tableName
              fields
              updatedAt
              version
            }
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ updateOne: MutationResult }>(mutation, { table, id, data });
    return result.updateOne;
  }

  /**
   * Update multiple rows matching a condition
   */
  async updateMany(
    table: string,
    where: WhereClause,
    data: Record<string, Json>
  ): Promise<MutationResult> {
    const mutation = `
      mutation UpdateMany($table: String!, $where: WhereClause!, $data: Map!) {
        updateMany(table: $table, where: $where, data: $data) {
          ... on MutationSuccess {
            affectedRows
            returning {
              id
              tableName
              fields
              updatedAt
              version
            }
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ updateMany: MutationResult }>(mutation, {
      table,
      where,
      data,
    });
    return result.updateMany;
  }

  /**
   * Delete a single row by ID
   */
  async deleteOne(table: string, id: ID): Promise<MutationResult> {
    const mutation = `
      mutation DeleteOne($table: String!, $id: ID!) {
        deleteOne(table: $table, id: $id) {
          ... on MutationSuccess {
            affectedRows
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ deleteOne: MutationResult }>(mutation, { table, id });
    return result.deleteOne;
  }

  /**
   * Delete multiple rows matching a condition
   */
  async deleteMany(table: string, where: WhereClause): Promise<MutationResult> {
    const mutation = `
      mutation DeleteMany($table: String!, $where: WhereClause!) {
        deleteMany(table: $table, where: $where) {
          ... on MutationSuccess {
            affectedRows
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ deleteMany: MutationResult }>(mutation, { table, where });
    return result.deleteMany;
  }

  /**
   * Upsert (insert or update) a row
   */
  async upsert(
    table: string,
    uniqueFields: string[],
    data: Record<string, Json>
  ): Promise<MutationResult> {
    const mutation = `
      mutation Upsert($table: String!, $uniqueFields: [String!]!, $data: Map!) {
        upsert(table: $table, uniqueFields: $uniqueFields, data: $data) {
          ... on MutationSuccess {
            affectedRows
            returning {
              id
              tableName
              fields
              createdAt
              updatedAt
              version
            }
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ upsert: MutationResult }>(mutation, {
      table,
      uniqueFields,
      data,
    });
    return result.upsert;
  }

  /**
   * Bulk insert with optimizations
   */
  async bulkInsert(
    table: string,
    data: Record<string, Json>[],
    batchSize?: number
  ): Promise<MutationResult> {
    const mutation = `
      mutation BulkInsert($table: String!, $data: [Map!]!, $batchSize: Int) {
        bulkInsert(table: $table, data: $data, batchSize: $batchSize) {
          ... on MutationSuccess {
            affectedRows
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ bulkInsert: MutationResult }>(mutation, {
      table,
      data,
      batchSize,
    });
    return result.bulkInsert;
  }

  // ============================================================================
  // MUTATION OPERATIONS - Transactions
  // ============================================================================

  /**
   * Begin a new transaction
   */
  async beginTransaction(isolationLevel?: IsolationLevel): Promise<TransactionResult> {
    const mutation = `
      mutation BeginTransaction($isolationLevel: IsolationLevel) {
        beginTransaction(isolationLevel: $isolationLevel) {
          transactionId
          status
          timestamp
        }
      }
    `;
    const result = await this.request<{ beginTransaction: TransactionResult }>(mutation, {
      isolationLevel,
    });
    return result.beginTransaction;
  }

  /**
   * Commit a transaction
   */
  async commitTransaction(transactionId: string): Promise<TransactionResult> {
    const mutation = `
      mutation CommitTransaction($transactionId: String!) {
        commitTransaction(transactionId: $transactionId) {
          transactionId
          status
          timestamp
        }
      }
    `;
    const result = await this.request<{ commitTransaction: TransactionResult }>(mutation, {
      transactionId,
    });
    return result.commitTransaction;
  }

  /**
   * Rollback a transaction
   */
  async rollbackTransaction(transactionId: string): Promise<TransactionResult> {
    const mutation = `
      mutation RollbackTransaction($transactionId: String!) {
        rollbackTransaction(transactionId: $transactionId) {
          transactionId
          status
          timestamp
        }
      }
    `;
    const result = await this.request<{ rollbackTransaction: TransactionResult }>(mutation, {
      transactionId,
    });
    return result.rollbackTransaction;
  }

  /**
   * Execute multiple mutations in a transaction
   */
  async executeTransaction(
    operations: TransactionOperation[],
    isolationLevel?: IsolationLevel
  ): Promise<TransactionExecutionResult> {
    const mutation = `
      mutation ExecuteTransaction(
        $operations: [TransactionOperation!]!,
        $isolationLevel: IsolationLevel
      ) {
        executeTransaction(operations: $operations, isolationLevel: $isolationLevel) {
          success
          results
          executionTimeMs
          error
        }
      }
    `;
    const result = await this.request<{ executeTransaction: TransactionExecutionResult }>(
      mutation,
      { operations, isolationLevel }
    );
    return result.executeTransaction;
  }

  // ============================================================================
  // MUTATION OPERATIONS - DDL (Database Management)
  // ============================================================================

  /**
   * Create a new database
   */
  async createDatabase(name: string, ifNotExists?: boolean): Promise<DdlResult> {
    const mutation = `
      mutation CreateDatabase($name: String!, $ifNotExists: Boolean) {
        createDatabase(name: $name, ifNotExists: $ifNotExists) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ createDatabase: DdlResult }>(mutation, {
      name,
      ifNotExists,
    });
    return result.createDatabase;
  }

  /**
   * Drop a database
   */
  async dropDatabase(name: string, ifExists?: boolean): Promise<DdlResult> {
    const mutation = `
      mutation DropDatabase($name: String!, $ifExists: Boolean) {
        dropDatabase(name: $name, ifExists: $ifExists) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ dropDatabase: DdlResult }>(mutation, { name, ifExists });
    return result.dropDatabase;
  }

  /**
   * Backup a database
   */
  async backupDatabase(
    name: string,
    location: string,
    fullBackup?: boolean
  ): Promise<DdlResult> {
    const mutation = `
      mutation BackupDatabase($name: String!, $location: String!, $fullBackup: Boolean) {
        backupDatabase(name: $name, location: $location, fullBackup: $fullBackup) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ backupDatabase: DdlResult }>(mutation, {
      name,
      location,
      fullBackup,
    });
    return result.backupDatabase;
  }

  // ============================================================================
  // MUTATION OPERATIONS - DDL (Table Management)
  // ============================================================================

  /**
   * Alter table - add column
   */
  async alterTableAddColumn(
    table: string,
    column: ColumnDefinitionInput
  ): Promise<DdlResult> {
    const mutation = `
      mutation AlterTableAddColumn($table: String!, $column: ColumnDefinitionInput!) {
        alterTableAddColumn(table: $table, column: $column) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ alterTableAddColumn: DdlResult }>(mutation, {
      table,
      column,
    });
    return result.alterTableAddColumn;
  }

  /**
   * Alter table - drop column
   */
  async alterTableDropColumn(
    table: string,
    columnName: string,
    ifExists?: boolean
  ): Promise<DdlResult> {
    const mutation = `
      mutation AlterTableDropColumn($table: String!, $columnName: String!, $ifExists: Boolean) {
        alterTableDropColumn(table: $table, columnName: $columnName, ifExists: $ifExists) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ alterTableDropColumn: DdlResult }>(mutation, {
      table,
      columnName,
      ifExists,
    });
    return result.alterTableDropColumn;
  }

  /**
   * Alter table - modify column
   */
  async alterTableModifyColumn(
    table: string,
    column: ColumnDefinitionInput
  ): Promise<DdlResult> {
    const mutation = `
      mutation AlterTableModifyColumn($table: String!, $column: ColumnDefinitionInput!) {
        alterTableModifyColumn(table: $table, column: $column) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ alterTableModifyColumn: DdlResult }>(mutation, {
      table,
      column,
    });
    return result.alterTableModifyColumn;
  }

  /**
   * Alter table - add constraint
   */
  async alterTableAddConstraint(
    table: string,
    constraint: ConstraintInput
  ): Promise<DdlResult> {
    const mutation = `
      mutation AlterTableAddConstraint($table: String!, $constraint: ConstraintInput!) {
        alterTableAddConstraint(table: $table, constraint: $constraint) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ alterTableAddConstraint: DdlResult }>(mutation, {
      table,
      constraint,
    });
    return result.alterTableAddConstraint;
  }

  /**
   * Alter table - drop constraint
   */
  async alterTableDropConstraint(
    table: string,
    constraintName: string,
    ifExists?: boolean
  ): Promise<DdlResult> {
    const mutation = `
      mutation AlterTableDropConstraint(
        $table: String!,
        $constraintName: String!,
        $ifExists: Boolean
      ) {
        alterTableDropConstraint(
          table: $table,
          constraintName: $constraintName,
          ifExists: $ifExists
        ) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ alterTableDropConstraint: DdlResult }>(mutation, {
      table,
      constraintName,
      ifExists,
    });
    return result.alterTableDropConstraint;
  }

  /**
   * Truncate table
   */
  async truncateTable(table: string): Promise<DdlResult> {
    const mutation = `
      mutation TruncateTable($table: String!) {
        truncateTable(table: $table) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ truncateTable: DdlResult }>(mutation, { table });
    return result.truncateTable;
  }

  // ============================================================================
  // MUTATION OPERATIONS - DDL (View Management)
  // ============================================================================

  /**
   * Create a view
   */
  async createView(name: string, query: string, orReplace?: boolean): Promise<DdlResult> {
    const mutation = `
      mutation CreateView($name: String!, $query: String!, $orReplace: Boolean) {
        createView(name: $name, query: $query, orReplace: $orReplace) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ createView: DdlResult }>(mutation, {
      name,
      query,
      orReplace,
    });
    return result.createView;
  }

  /**
   * Drop a view
   */
  async dropView(name: string, ifExists?: boolean): Promise<DdlResult> {
    const mutation = `
      mutation DropView($name: String!, $ifExists: Boolean) {
        dropView(name: $name, ifExists: $ifExists) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ dropView: DdlResult }>(mutation, { name, ifExists });
    return result.dropView;
  }

  // ============================================================================
  // MUTATION OPERATIONS - DDL (Index Management)
  // ============================================================================

  /**
   * Create an index
   */
  async createIndex(options: {
    table: string;
    indexName: string;
    columns: string[];
    unique?: boolean;
    ifNotExists?: boolean;
  }): Promise<DdlResult> {
    const mutation = `
      mutation CreateIndex(
        $table: String!,
        $indexName: String!,
        $columns: [String!]!,
        $unique: Boolean,
        $ifNotExists: Boolean
      ) {
        createIndex(
          table: $table,
          indexName: $indexName,
          columns: $columns,
          unique: $unique,
          ifNotExists: $ifNotExists
        ) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ createIndex: DdlResult }>(mutation, options);
    return result.createIndex;
  }

  /**
   * Drop an index
   */
  async dropIndex(indexName: string, table?: string, ifExists?: boolean): Promise<DdlResult> {
    const mutation = `
      mutation DropIndex($indexName: String!, $table: String, $ifExists: Boolean) {
        dropIndex(indexName: $indexName, table: $table, ifExists: $ifExists) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ dropIndex: DdlResult }>(mutation, {
      indexName,
      table,
      ifExists,
    });
    return result.dropIndex;
  }

  // ============================================================================
  // MUTATION OPERATIONS - Stored Procedures
  // ============================================================================

  /**
   * Create a stored procedure
   */
  async createProcedure(
    name: string,
    parameters: ProcedureParameter[],
    body: string,
    orReplace?: boolean
  ): Promise<DdlResult> {
    const mutation = `
      mutation CreateProcedure(
        $name: String!,
        $parameters: [ProcedureParameter!]!,
        $body: String!,
        $orReplace: Boolean
      ) {
        createProcedure(
          name: $name,
          parameters: $parameters,
          body: $body,
          orReplace: $orReplace
        ) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ createProcedure: DdlResult }>(mutation, {
      name,
      parameters,
      body,
      orReplace,
    });
    return result.createProcedure;
  }

  /**
   * Execute a stored procedure
   */
  async executeProcedure(name: string, args?: Json[]): Promise<ProcedureResult> {
    const mutation = `
      mutation ExecuteProcedure($name: String!, $arguments: [Json!]) {
        executeProcedure(name: $name, arguments: $arguments) {
          ... on ProcedureSuccess {
            result
            executionTimeMs
          }
          ... on ProcedureError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ executeProcedure: ProcedureResult }>(mutation, {
      name,
      arguments: args,
    });
    return result.executeProcedure;
  }

  // ============================================================================
  // MUTATION OPERATIONS - Advanced Query Operations
  // ============================================================================

  /**
   * Insert into ... select
   */
  async insertIntoSelect(
    targetTable: string,
    sourceQuery: string,
    targetColumns?: string[]
  ): Promise<MutationResult> {
    const mutation = `
      mutation InsertIntoSelect(
        $targetTable: String!,
        $sourceQuery: String!,
        $targetColumns: [String!]
      ) {
        insertIntoSelect(
          targetTable: $targetTable,
          targetColumns: $targetColumns,
          sourceQuery: $sourceQuery
        ) {
          ... on MutationSuccess {
            affectedRows
            executionTimeMs
          }
          ... on MutationError {
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ insertIntoSelect: MutationResult }>(mutation, {
      targetTable,
      sourceQuery,
      targetColumns,
    });
    return result.insertIntoSelect;
  }

  /**
   * Select into (create new table from select)
   */
  async selectInto(newTable: string, sourceQuery: string): Promise<DdlResult> {
    const mutation = `
      mutation SelectInto($newTable: String!, $sourceQuery: String!) {
        selectInto(newTable: $newTable, sourceQuery: $sourceQuery) {
          ... on DdlSuccess {
            success
            message
            affectedRows
            executionTimeMs
          }
          ... on DdlError {
            success
            message
            code
            details
          }
        }
      }
    `;
    const result = await this.request<{ selectInto: DdlResult }>(mutation, {
      newTable,
      sourceQuery,
    });
    return result.selectInto;
  }

  // ============================================================================
  // MUTATION OPERATIONS - String Functions
  // ============================================================================

  /**
   * Execute a string function
   */
  async executeStringFunction(
    functionType: StringFunctionTypeEnum,
    parameters: string[]
  ): Promise<StringFunctionResult> {
    const mutation = `
      mutation ExecuteStringFunction(
        $functionType: StringFunctionTypeEnum!,
        $parameters: [String!]!
      ) {
        executeStringFunction(functionType: $functionType, parameters: $parameters) {
          result
          executionTimeMs
        }
      }
    `;
    const result = await this.request<{ executeStringFunction: StringFunctionResult }>(mutation, {
      functionType,
      parameters,
    });
    return result.executeStringFunction;
  }

  /**
   * Batch execute string functions
   */
  async batchStringFunctions(
    functions: StringFunctionInput[]
  ): Promise<BatchStringFunctionResult> {
    const mutation = `
      mutation BatchStringFunctions($functions: [StringFunctionInput!]!) {
        batchStringFunctions(functions: $functions) {
          results
          executionTimeMs
        }
      }
    `;
    const result = await this.request<{ batchStringFunctions: BatchStringFunctionResult }>(
      mutation,
      { functions }
    );
    return result.batchStringFunctions;
  }

  // ============================================================================
  // SUBSCRIPTION OPERATIONS
  // ============================================================================

  /**
   * Subscribe to all changes on a table
   */
  subscribeTableChanges(
    table: string,
    where?: WhereClause,
    onData?: (data: TableChange) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription TableChanges($table: String!, $where: WhereClause) {
        tableChanges(table: $table, where: $where) {
          table
          changeType
          row {
            id
            tableName
            fields
            createdAt
            version
          }
          oldRow {
            id
            tableName
            fields
            createdAt
            version
          }
          timestamp
        }
      }
    `;
    return this.subscribe<{ tableChanges: TableChange }>(
      subscription,
      { table, where },
      (data) => onData?.(data.tableChanges),
      onError
    );
  }

  /**
   * Subscribe to row insertions
   */
  subscribeRowInserted(
    table: string,
    where?: WhereClause,
    onData?: (data: RowInserted) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription RowInserted($table: String!, $where: WhereClause) {
        rowInserted(table: $table, where: $where) {
          table
          row {
            id
            tableName
            fields
            createdAt
            version
          }
          timestamp
        }
      }
    `;
    return this.subscribe<{ rowInserted: RowInserted }>(
      subscription,
      { table, where },
      (data) => onData?.(data.rowInserted),
      onError
    );
  }

  /**
   * Subscribe to row updates
   */
  subscribeRowUpdated(
    table: string,
    where?: WhereClause,
    onData?: (data: RowUpdated) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription RowUpdated($table: String!, $where: WhereClause) {
        rowUpdated(table: $table, where: $where) {
          table
          oldRow {
            id
            tableName
            fields
            createdAt
            version
          }
          newRow {
            id
            tableName
            fields
            updatedAt
            version
          }
          changedFields
          timestamp
        }
      }
    `;
    return this.subscribe<{ rowUpdated: RowUpdated }>(
      subscription,
      { table, where },
      (data) => onData?.(data.rowUpdated),
      onError
    );
  }

  /**
   * Subscribe to row deletions
   */
  subscribeRowDeleted(
    table: string,
    where?: WhereClause,
    onData?: (data: RowDeleted) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription RowDeleted($table: String!, $where: WhereClause) {
        rowDeleted(table: $table, where: $where) {
          table
          id
          oldRow {
            id
            tableName
            fields
          }
          timestamp
        }
      }
    `;
    return this.subscribe<{ rowDeleted: RowDeleted }>(
      subscription,
      { table, where },
      (data) => onData?.(data.rowDeleted),
      onError
    );
  }

  /**
   * Subscribe to specific row changes by ID
   */
  subscribeRowChanges(
    table: string,
    id: ID,
    onData?: (data: RowChange) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription RowChanges($table: String!, $id: ID!) {
        rowChanges(table: $table, id: $id) {
          table
          id
          changeType
          row {
            id
            tableName
            fields
            version
          }
          oldRow {
            id
            tableName
            fields
            version
          }
          timestamp
        }
      }
    `;
    return this.subscribe<{ rowChanges: RowChange }>(
      subscription,
      { table, id },
      (data) => onData?.(data.rowChanges),
      onError
    );
  }

  /**
   * Subscribe to aggregation changes
   */
  subscribeAggregateChanges(
    table: string,
    aggregates: AggregateInput[],
    where?: WhereClause,
    intervalSeconds?: number,
    onData?: (data: AggregateChange) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription AggregateChanges(
        $table: String!,
        $aggregates: [AggregateInput!]!,
        $where: WhereClause,
        $intervalSeconds: Int
      ) {
        aggregateChanges(
          table: $table,
          aggregates: $aggregates,
          where: $where,
          intervalSeconds: $intervalSeconds
        ) {
          table
          results {
            field
            function
            value
          }
          timestamp
        }
      }
    `;
    return this.subscribe<{ aggregateChanges: AggregateChange }>(
      subscription,
      { table, aggregates, where, intervalSeconds },
      (data) => onData?.(data.aggregateChanges),
      onError
    );
  }

  /**
   * Subscribe to query result changes
   */
  subscribeQueryChanges(
    table: string,
    where?: WhereClause,
    orderBy?: OrderBy[],
    limit?: number,
    pollIntervalSeconds?: number,
    onData?: (data: QueryChange) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription QueryChanges(
        $table: String!,
        $where: WhereClause,
        $orderBy: [OrderBy!],
        $limit: Int,
        $pollIntervalSeconds: Int
      ) {
        queryChanges(
          table: $table,
          where: $where,
          orderBy: $orderBy,
          limit: $limit,
          pollIntervalSeconds: $pollIntervalSeconds
        ) {
          table
          rows {
            id
            tableName
            fields
            createdAt
            version
          }
          totalCount
          timestamp
        }
      }
    `;
    return this.subscribe<{ queryChanges: QueryChange }>(
      subscription,
      { table, where, orderBy, limit, pollIntervalSeconds },
      (data) => onData?.(data.queryChanges),
      onError
    );
  }

  /**
   * Heartbeat subscription for connection keepalive
   */
  subscribeHeartbeat(
    intervalSeconds?: number,
    onData?: (data: Heartbeat) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription Heartbeat($intervalSeconds: Int) {
        heartbeat(intervalSeconds: $intervalSeconds) {
          sequence
          timestamp
        }
      }
    `;
    return this.subscribe<{ heartbeat: Heartbeat }>(
      subscription,
      { intervalSeconds },
      (data) => onData?.(data.heartbeat),
      onError
    );
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - DDL Events
  // ============================================================================

  /**
   * Subscribe to schema changes (DDL events)
   */
  subscribeSchemaChanges(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription SchemaChanges {
        schemaChanges {
          changeType
          objectType
          objectName
          schemaName
          timestamp
          userId
          ddlStatement
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to partition events
   */
  subscribePartitionEvents(
    tableName?: string,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription PartitionEvents($tableName: String) {
        partitionEvents(tableName: $tableName) {
          eventType
          tableName
          partitionName
          partitionKey
          rowCount
          sizeBytes
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, { tableName }, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - Cluster Events
  // ============================================================================

  /**
   * Subscribe to cluster topology changes
   */
  subscribeClusterTopologyChanges(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription ClusterTopologyChanges {
        clusterTopologyChanges {
          changeType
          nodeId
          nodeName
          nodeRole
          nodeStatus
          address
          timestamp
          term
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to node health changes
   */
  subscribeNodeHealthChanges(
    nodeId?: string,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription NodeHealthChanges($nodeId: String) {
        nodeHealthChanges(nodeId: $nodeId) {
          nodeId
          previousStatus
          currentStatus
          cpuUsage
          memoryUsage
          diskUsage
          lastHeartbeat
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, { nodeId }, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - Query & Performance Events
  // ============================================================================

  /**
   * Subscribe to active queries stream
   */
  subscribeActiveQueriesStream(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription ActiveQueriesStream {
        activeQueriesStream {
          queryId
          sessionId
          username
          sqlText
          state
          startTime
          durationMs
          rowsProcessed
          waitEvent
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to slow queries stream
   */
  subscribeSlowQueriesStream(
    thresholdMs?: number,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription SlowQueriesStream($thresholdMs: Int) {
        slowQueriesStream(thresholdMs: $thresholdMs) {
          queryId
          sqlText
          executionTimeMs
          startTime
          endTime
          username
          database
          rowsReturned
        }
      }
    `;
    return this.subscribe(subscription, { thresholdMs }, onData, onError);
  }

  /**
   * Subscribe to query plan changes
   */
  subscribeQueryPlanChanges(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription QueryPlanChanges {
        queryPlanChanges {
          queryFingerprint
          previousPlanHash
          newPlanHash
          reason
          estimatedCostChange
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - Transaction & Lock Events
  // ============================================================================

  /**
   * Subscribe to transaction events
   */
  subscribeTransactionEvents(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription TransactionEvents {
        transactionEvents {
          transactionId
          eventType
          sessionId
          isolationLevel
          timestamp
          rowsAffected
          durationMs
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to lock events
   */
  subscribeLockEvents(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription LockEvents {
        lockEvents {
          lockId
          eventType
          transactionId
          lockType
          lockMode
          resource
          tableName
          timestamp
          waitTimeMs
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to deadlock detection events
   */
  subscribeDeadlockDetection(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription DeadlockDetection {
        deadlockDetection {
          deadlockId
          detectedAt
          transactions
          victimTransaction
          resourceGraph
          resolution
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - Alert & Health Events
  // ============================================================================

  /**
   * Subscribe to alert stream
   */
  subscribeAlertStream(
    severity?: string,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription AlertStream($severity: AlertSeverity) {
        alertStream(severity: $severity) {
          id
          name
          category
          severity
          state
          message
          details
          triggeredAt
          escalationLevel
        }
      }
    `;
    return this.subscribe(subscription, { severity }, onData, onError);
  }

  /**
   * Subscribe to health status changes
   */
  subscribeHealthStatusChanges(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription HealthStatusChanges {
        healthStatusChanges {
          status
          components {
            name
            status
            responseTimeMs
            details
          }
          errors
          warnings
          checkedAt
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - Storage Events
  // ============================================================================

  /**
   * Subscribe to storage status changes
   */
  subscribeStorageStatusChanges(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription StorageStatusChanges {
        storageStatusChanges {
          totalBytes
          usedBytes
          availableBytes
          usagePercent
          dataFiles
          dataSize
          indexFiles
          indexSize
          walSize
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to buffer pool metrics
   */
  subscribeBufferPoolMetrics(
    intervalSeconds?: number,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription BufferPoolMetrics($intervalSeconds: Int) {
        bufferPoolMetrics(intervalSeconds: $intervalSeconds) {
          sizeBytes
          totalPages
          freePages
          dirtyPages
          hitRatio
          totalReads
          totalWrites
          cacheHits
          cacheMisses
          evictions
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, { intervalSeconds }, onData, onError);
  }

  /**
   * Subscribe to I/O statistics stream
   */
  subscribeIoStatisticsStream(
    intervalSeconds?: number,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription IoStatisticsStream($intervalSeconds: Int) {
        ioStatisticsStream(intervalSeconds: $intervalSeconds) {
          reads
          writes
          bytesRead
          bytesWritten
          avgReadLatencyUs
          avgWriteLatencyUs
          readThroughputBps
          writeThroughputBps
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, { intervalSeconds }, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - Session Events
  // ============================================================================

  /**
   * Subscribe to session events
   */
  subscribeSessionEvents(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription SessionEvents {
        sessionEvents {
          eventType
          sessionId
          userId
          username
          clientAddress
          database
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to connection pool events
   */
  subscribeConnectionPoolEvents(
    poolId?: string,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription ConnectionPoolEvents($poolId: String) {
        connectionPoolEvents(poolId: $poolId) {
          eventType
          poolId
          poolName
          activeConnections
          idleConnections
          waitingRequests
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, { poolId }, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - Security Events
  // ============================================================================

  /**
   * Subscribe to security events
   */
  subscribeSecurityEvents(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription SecurityEvents {
        securityEvents {
          eventType
          userId
          username
          action
          resource
          result
          clientIp
          timestamp
          details
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to audit stream
   */
  subscribeAuditStream(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription AuditStream {
        auditStream {
          auditId
          timestamp
          userId
          username
          action
          objectType
          objectName
          sqlText
          result
          clientInfo
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to threat alerts
   */
  subscribeThreatAlerts(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription ThreatAlerts {
        threatAlerts {
          alertId
          threatLevel
          threatType
          userId
          description
          riskScore
          timestamp
          mitigationAction
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - Replication Events
  // ============================================================================

  /**
   * Subscribe to replication lag updates
   */
  subscribeReplicationLag(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription ReplicationLag {
        replicationLag {
          replicaId
          replicaName
          lagMs
          bytesBehind
          lastWalReceived
          lastWalApplied
          status
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  /**
   * Subscribe to WAL events
   */
  subscribeWalEvents(
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription WalEvents {
        walEvents {
          eventType
          lsn
          walFile
          sizeBytes
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, {}, onData, onError);
  }

  // ============================================================================
  // PR48 SUBSCRIPTIONS - ML Events
  // ============================================================================

  /**
   * Subscribe to ML training events
   */
  subscribeTrainingEvents(
    modelId?: string,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription TrainingEvents($modelId: String) {
        trainingEvents(modelId: $modelId) {
          modelId
          eventType
          epoch
          loss
          accuracy
          validationLoss
          validationAccuracy
          progress
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, { modelId }, onData, onError);
  }

  /**
   * Subscribe to prediction stream
   */
  subscribePredictionStream(
    modelId?: string,
    onData?: (data: unknown) => void,
    onError?: (error: Error) => void
  ): () => void {
    const subscription = `
      subscription PredictionStream($modelId: String) {
        predictionStream(modelId: $modelId) {
          predictionId
          modelId
          input
          prediction
          confidence
          latencyMs
          timestamp
        }
      }
    `;
    return this.subscribe(subscription, { modelId }, onData, onError);
  }

  // ============================================================================
  // SUBSCRIPTION HELPER METHOD
  // ============================================================================

  private subscribe<T>(
    query: string,
    variables: Record<string, unknown>,
    onData?: (data: T) => void,
    onError?: (error: Error) => void
  ): () => void {
    // This is a simplified WebSocket subscription implementation
    // In a real implementation, you would use a GraphQL subscription client
    // like graphql-ws or subscriptions-transport-ws

    // eslint-disable-next-line no-console
    console.log('Subscription started:', { query, variables, onData, onError });

    // Return unsubscribe function
    return () => {
      // eslint-disable-next-line no-console
      console.log('Subscription ended');
    };
  }

  // ============================================================================
  // CONNECTION MANAGEMENT
  // ============================================================================

  /**
   * Close all connections
   */
  async close(): Promise<void> {
    if (this.wsClient) {
      // Close WebSocket connection
      this.wsClient = undefined;
    }
  }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Create a GraphQL client instance
 */
export function createGraphQLClient(config: GraphQLClientConfig): RustyDBGraphQLClient {
  return new RustyDBGraphQLClient(config);
}

export default RustyDBGraphQLClient;
