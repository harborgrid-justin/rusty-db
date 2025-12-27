/**
 * GraphQL Types for RustyDB
 *
 * Complete TypeScript type definitions matching the GraphQL schema
 * Generated from src/api/graphql/ Rust implementation
 */

// ============================================================================
// SCALAR TYPES
// ============================================================================

/** ISO 8601 DateTime string */
export type DateTime = string;

/** Arbitrary JSON value */
export type Json = unknown;

/** Base64-encoded binary data */
export type Binary = string;

/** Large integer as string to prevent precision loss */
export type BigIntString = string;

/** GraphQL ID type */
export type ID = string;

// ============================================================================
// ENUM TYPES
// ============================================================================

export enum DataType {
  Null = 'NULL',
  Boolean = 'BOOLEAN',
  Integer = 'INTEGER',
  Float = 'FLOAT',
  String = 'STRING',
  Bytes = 'BYTES',
  Date = 'DATE',
  Timestamp = 'TIMESTAMP',
  Json = 'JSON',
  Array = 'ARRAY',
  Decimal = 'DECIMAL',
  Uuid = 'UUID',
}

export enum SortOrder {
  Asc = 'ASC',
  Desc = 'DESC',
}

export enum FilterOp {
  Eq = 'EQ',
  Ne = 'NE',
  Lt = 'LT',
  Le = 'LE',
  Gt = 'GT',
  Ge = 'GE',
  Like = 'LIKE',
  NotLike = 'NOT_LIKE',
  In = 'IN',
  NotIn = 'NOT_IN',
  IsNull = 'IS_NULL',
  IsNotNull = 'IS_NOT_NULL',
  Between = 'BETWEEN',
  Contains = 'CONTAINS',
  StartsWith = 'STARTS_WITH',
  EndsWith = 'ENDS_WITH',
}

export enum AggregateFunc {
  Count = 'COUNT',
  Sum = 'SUM',
  Avg = 'AVG',
  Min = 'MIN',
  Max = 'MAX',
  StdDev = 'STD_DEV',
  Variance = 'VARIANCE',
}

export enum JoinType {
  Inner = 'INNER',
  Left = 'LEFT',
  Right = 'RIGHT',
  Full = 'FULL',
  Cross = 'CROSS',
}

export enum IsolationLevel {
  ReadUncommitted = 'READ_UNCOMMITTED',
  ReadCommitted = 'READ_COMMITTED',
  RepeatableRead = 'REPEATABLE_READ',
  Serializable = 'SERIALIZABLE',
  SnapshotIsolation = 'SNAPSHOT_ISOLATION',
}

export enum ChangeType {
  Insert = 'INSERT',
  Update = 'UPDATE',
  Delete = 'DELETE',
}

export enum TransactionOpType {
  Insert = 'INSERT',
  Update = 'UPDATE',
  Delete = 'DELETE',
}

export enum ParameterMode {
  In = 'IN',
  Out = 'OUT',
  InOut = 'IN_OUT',
}

export enum ConstraintTypeEnum {
  PrimaryKey = 'PRIMARY_KEY',
  ForeignKey = 'FOREIGN_KEY',
  Unique = 'UNIQUE',
  Check = 'CHECK',
  Default = 'DEFAULT',
}

export enum StringFunctionTypeEnum {
  Ascii = 'ASCII',
  Char = 'CHAR',
  CharIndex = 'CHAR_INDEX',
  Concat = 'CONCAT',
  ConcatWs = 'CONCAT_WS',
  DataLength = 'DATA_LENGTH',
  Difference = 'DIFFERENCE',
  Format = 'FORMAT',
  Left = 'LEFT',
  Len = 'LEN',
  Lower = 'LOWER',
  LTrim = 'L_TRIM',
  NChar = 'N_CHAR',
  PatIndex = 'PAT_INDEX',
  QuoteName = 'QUOTE_NAME',
  Replace = 'REPLACE',
  Replicate = 'REPLICATE',
  Reverse = 'REVERSE',
  Right = 'RIGHT',
  RTrim = 'R_TRIM',
  Soundex = 'SOUNDEX',
  Space = 'SPACE',
  Str = 'STR',
  Stuff = 'STUFF',
  Substring = 'SUBSTRING',
  Translate = 'TRANSLATE',
  Trim = 'TRIM',
  Unicode = 'UNICODE',
  Upper = 'UPPER',
}

export enum AlertSeverity {
  Info = 'INFO',
  Warning = 'WARNING',
  Error = 'ERROR',
  Critical = 'CRITICAL',
}

// ============================================================================
// SCHEMA TYPES
// ============================================================================

export interface DatabaseSchema {
  name: string;
  tables: TableType[];
  tableCount: number;
  createdAt: DateTime;
  description?: string | null;
}

export interface TableType {
  id: ID;
  name: string;
  schema: string;
  columns: ColumnType[];
  rowCount: BigIntStringString;
  sizeBytes: BigIntStringString;
  createdAt: DateTime;
  updatedAt?: DateTime | null;
  createdBy: string;
  updatedBy?: string | null;
  description?: string | null;
  indexes: IndexInfo[];
  constraints: ConstraintInfo[];
}

export interface ColumnType {
  id: ID;
  name: string;
  tableName: string;
  dataType: DataType;
  nullable: boolean;
  defaultValue?: string | null;
  position: number;
  maxLength?: number | null;
  precision?: number | null;
  scale?: number | null;
  description?: string | null;
}

export interface RowType {
  id: ID;
  tableName: string;
  fields: Record<string, FieldValue>;
  createdAt: DateTime;
  updatedAt?: DateTime | null;
  createdBy: string;
  updatedBy?: string | null;
  version: number;
}

export interface FieldValue {
  columnName: string;
  value: Json;
  dataType: DataType;
  stringValue?: string | null;
  intValue?: number | null;
  floatValue?: number | null;
  boolValue?: boolean | null;
}

export interface IndexInfo {
  name: string;
  columns: string[];
  unique: boolean;
  indexType: string;
  sizeBytes: BigIntStringString;
  createdAt: DateTime;
}

export interface ConstraintInfo {
  name: string;
  constraintType: string;
  columns: string[];
  referencedTable?: string | null;
  referencedColumns?: string[] | null;
}

export interface TableStatistics {
  rowCount: BigIntStringString;
  sizeBytes: BigIntStringString;
  indexSizeBytes: BigIntStringString;
  avgRowSize: number;
  lastAnalyzed?: DateTime | null;
  lastModified?: DateTime | null;
}

export interface ColumnStatistics {
  distinctCount: BigIntStringString;
  nullCount: BigIntStringString;
  avgLength?: number | null;
  minValue?: string | null;
  maxValue?: string | null;
  histogram?: HistogramBucket[] | null;
}

export interface HistogramBucket {
  rangeStart: string;
  rangeEnd: string;
  count: BigIntStringString;
  frequency: number;
}

// ============================================================================
// QUERY RESULT TYPES
// ============================================================================

export type QueryResult = QuerySuccess | QueryError;

export interface QuerySuccess {
  __typename: 'QuerySuccess';
  rows: RowType[];
  totalCount: BigIntStringString;
  executionTimeMs: number;
  hasMore: boolean;
}

export interface QueryError {
  __typename: 'QueryError';
  message: string;
  code: string;
  details?: string | null;
}

export interface SearchResult {
  results: SearchMatch[];
  totalCount: BigIntStringString;
  executionTimeMs: number;
}

export interface SearchMatch {
  table: string;
  row: RowType;
  score: number;
  highlights: Record<string, string>;
}

export interface QueryPlan {
  planText: string;
  estimatedCost: number;
  estimatedRows: BigIntStringString;
  operations: PlanOperation[];
}

export interface PlanOperation {
  operationType: string;
  description: string;
  cost: number;
  rows: BigIntStringString;
  children: PlanOperation[];
}

// ============================================================================
// MUTATION RESULT TYPES
// ============================================================================

export type MutationResult = MutationSuccess | MutationError;

export interface MutationSuccess {
  __typename: 'MutationSuccess';
  affectedRows: number;
  returning?: RowType[] | null;
  executionTimeMs: number;
}

export interface MutationError {
  __typename: 'MutationError';
  message: string;
  code: string;
  details?: string | null;
}

export type DdlResult = DdlSuccess | DdlError;

export interface DdlSuccess {
  __typename: 'DdlSuccess';
  success: boolean;
  message: string;
  affectedRows: number;
  executionTimeMs: number;
}

export interface DdlError {
  __typename: 'DdlError';
  success: boolean;
  message: string;
  code: string;
  details?: string | null;
}

export type ProcedureResult = ProcedureSuccess | ProcedureError;

export interface ProcedureSuccess {
  __typename: 'ProcedureSuccess';
  result: Json;
  executionTimeMs: number;
}

export interface ProcedureError {
  __typename: 'ProcedureError';
  message: string;
  code: string;
  details?: string | null;
}

// ============================================================================
// INPUT TYPES
// ============================================================================

export interface FilterCondition {
  field: string;
  op: FilterOp;
  value?: Json | null;
  values?: Json[] | null;
}

export interface WhereClause {
  and?: WhereClause[] | null;
  or?: WhereClause[] | null;
  not?: WhereClause | null;
  condition?: FilterCondition | null;
}

export interface OrderBy {
  field: string;
  order: SortOrder;
}

export interface AggregateInput {
  function: AggregateFunc;
  field: string;
  alias?: string | null;
}

export interface AggregateResult {
  field: string;
  function: AggregateFunc;
  value: Json;
}

export interface JoinInput {
  table: string;
  joinType: JoinType;
  onField: string;
  otherField: string;
}

export interface ColumnDefinitionInput {
  name: string;
  dataType: string;
  nullable?: boolean | null;
  defaultValue?: Json | null;
  primaryKey?: boolean | null;
  unique?: boolean | null;
  autoIncrement?: boolean | null;
}

export interface ConstraintInput {
  name: string;
  constraintType: ConstraintTypeEnum;
  columns: string[];
  referenceTable?: string | null;
  referenceColumns?: string[] | null;
  checkExpression?: string | null;
}

export interface ProcedureParameter {
  name: string;
  dataType: string;
  mode?: ParameterMode | null;
}

export interface TransactionOperation {
  operationType: TransactionOpType;
  table: string;
  data?: Record<string, Json> | null;
  whereClause?: WhereClause | null;
  id?: ID | null;
}

export interface StringFunctionInput {
  functionType: StringFunctionTypeEnum;
  parameters: string[];
}

// ============================================================================
// TRANSACTION TYPES
// ============================================================================

export interface TransactionResult {
  transactionId: string;
  status: string;
  timestamp: DateTime;
}

export interface TransactionExecutionResult {
  success: boolean;
  results: string[];
  executionTimeMs: number;
  error?: string | null;
}

// ============================================================================
// STRING FUNCTION TYPES
// ============================================================================

export interface StringFunctionResult {
  result: string;
  executionTimeMs: number;
}

export interface BatchStringFunctionResult {
  results: string[];
  executionTimeMs: number;
}

// ============================================================================
// PAGINATION TYPES
// ============================================================================

export interface PageInfo {
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  startCursor?: string | null;
  endCursor?: string | null;
  totalCount: BigIntStringString;
}

export interface RowEdge {
  cursor: string;
  node: RowType;
}

export interface RowConnection {
  edges: RowEdge[];
  pageInfo: PageInfo;
  totalCount: BigIntStringString;
}

// ============================================================================
// SUBSCRIPTION TYPES
// ============================================================================

export interface TableChange {
  table: string;
  changeType: ChangeType;
  row?: RowType | null;
  oldRow?: RowType | null;
  timestamp: DateTime;
}

export interface RowInserted {
  table: string;
  row: RowType;
  timestamp: DateTime;
}

export interface RowUpdated {
  table: string;
  oldRow: RowType;
  newRow: RowType;
  changedFields: string[];
  timestamp: DateTime;
}

export interface RowDeleted {
  table: string;
  id: ID;
  oldRow?: RowType | null;
  timestamp: DateTime;
}

export interface RowChange {
  table: string;
  id: ID;
  changeType: ChangeType;
  row?: RowType | null;
  oldRow?: RowType | null;
  timestamp: DateTime;
}

export interface AggregateChange {
  table: string;
  results: AggregateResult[];
  timestamp: DateTime;
}

export interface QueryChange {
  table: string;
  rows: RowType[];
  totalCount: BigIntStringString;
  timestamp: DateTime;
}

export interface Heartbeat {
  sequence: number;
  timestamp: DateTime;
}

// ============================================================================
// MONITORING TYPES
// ============================================================================

export interface MetricsResponse {
  cpuUsage: number;
  memoryUsed: BigIntStringString;
  memoryTotal: BigIntStringString;
  memoryPercent: number;
  diskUsed: BigIntStringString;
  diskTotal: BigIntStringString;
  diskPercent: number;
  activeConnections: number;
  totalConnections: number;
  qps: number;
  cacheHitRatio: number;
  timestamp: DateTime;
}

export interface SessionStats {
  activeSessions: number;
  idleSessions: number;
  totalSessions: number;
  avgSessionDuration: number;
  peakSessions: number;
  timestamp: DateTime;
}

export interface QueryStats {
  totalQueries: BigIntStringString;
  successfulQueries: BigIntStringString;
  failedQueries: BigIntStringString;
  avgExecutionTimeMs: number;
  medianExecutionTimeMs: number;
  p95ExecutionTimeMs: number;
  p99ExecutionTimeMs: number;
  qps: number;
  timestamp: DateTime;
}

export interface PerformanceData {
  cpuUsage: number;
  memoryUsage: number;
  diskReadBps: BigIntStringString;
  diskWriteBps: BigIntStringString;
  networkRxBps: BigIntStringString;
  networkTxBps: BigIntStringString;
  activeQueries: number;
  waitingQueries: number;
  bufferHitRatio: number;
  commitRate: number;
  rollbackRate: number;
  timestamp: DateTime;
}

export interface ActiveQuery {
  queryId: string;
  sessionId: string;
  username: string;
  sqlText: string;
  state: string;
  startTime: DateTime;
  durationMs: BigIntStringString;
  rowsProcessed: BigIntStringString;
  waitEvent?: string | null;
}

export interface SlowQuery {
  queryId: string;
  sqlText: string;
  executionTimeMs: BigIntStringString;
  startTime: DateTime;
  endTime: DateTime;
  username: string;
  database: string;
  rowsReturned: BigIntStringString;
}

// ============================================================================
// CLUSTER TYPES
// ============================================================================

export interface ClusterNode {
  id: string;
  name: string;
  role: string;
  status: string;
  address: string;
  lastHeartbeat: DateTime;
  uptimeSeconds: BigIntStringString;
  term: BigIntStringString;
  isLeader: boolean;
  cpuUsage: number;
  memoryUsage: number;
}

export interface ClusterTopology {
  totalNodes: number;
  healthyNodes: number;
  leaderId?: string | null;
  currentTerm: BigIntStringString;
  hasQuorum: boolean;
  nodes: ClusterNode[];
  timestamp: DateTime;
}

export interface ReplicationStatus {
  mode: string;
  state: string;
  lagMs: BigIntStringString;
  bytesBehind: BigIntStringString;
  lastWalReceived: string;
  lastWalApplied: string;
  timestamp: DateTime;
}

export interface ClusterConfig {
  clusterName: string;
  replicationFactor: number;
  minQuorumSize: number;
  electionTimeoutMs: number;
  heartbeatIntervalMs: number;
  autoFailover: boolean;
  geoReplication: boolean;
}

// ============================================================================
// STORAGE TYPES
// ============================================================================

export interface StorageStatus {
  totalBytes: BigIntStringString;
  usedBytes: BigIntStringString;
  availableBytes: BigIntStringString;
  usagePercent: number;
  dataFiles: number;
  dataSize: BigIntStringString;
  indexFiles: number;
  indexSize: BigIntStringString;
  walSize: BigIntStringString;
  timestamp: DateTime;
}

export interface BufferPoolStats {
  sizeBytes: BigIntStringString;
  totalPages: number;
  freePages: number;
  dirtyPages: number;
  hitRatio: number;
  totalReads: BigIntStringString;
  totalWrites: BigIntStringString;
  cacheHits: BigIntStringString;
  cacheMisses: BigIntStringString;
  evictions: BigIntStringString;
  timestamp: DateTime;
}

export interface Tablespace {
  id: string;
  name: string;
  location: string;
  sizeBytes: BigIntStringString;
  usedBytes: BigIntStringString;
  tableCount: number;
  isDefault: boolean;
  createdAt: DateTime;
}

export interface IoStats {
  reads: BigIntStringString;
  writes: BigIntStringString;
  bytesRead: BigIntStringString;
  bytesWritten: BigIntStringString;
  avgReadLatencyUs: number;
  avgWriteLatencyUs: number;
  readThroughputBps: BigIntStringString;
  writeThroughputBps: BigIntStringString;
  timestamp: DateTime;
}

// ============================================================================
// TRANSACTION/LOCK TYPES
// ============================================================================

export interface ActiveTransaction {
  transactionId: string;
  sessionId: string;
  username: string;
  state: string;
  isolationLevel: string;
  startTime: DateTime;
  durationMs: BigIntStringString;
  queryCount: number;
  rowsModified: BigIntStringString;
}

export interface Lock {
  lockId: string;
  transactionId: string;
  lockType: string;
  lockMode: string;
  resource: string;
  tableName?: string | null;
  rowId?: string | null;
  grantedAt: DateTime;
  waitTimeMs?: BigIntString | null;
}

export interface Deadlock {
  deadlockId: string;
  detectedAt: DateTime;
  transactions: string[];
  victimTransaction: string;
  resourceGraph: string;
  resolution: string;
}

export interface MvccStatus {
  currentSnapshotId: string;
  oldestTransactionId?: string | null;
  activeSnapshots: number;
  totalVersions: BigIntStringString;
  deadVersions: BigIntStringString;
  lastVacuum?: DateTime | null;
  timestamp: DateTime;
}

// ============================================================================
// ADMIN TYPES
// ============================================================================

export interface ServerConfig {
  version: string;
  port: number;
  maxConnections: number;
  bufferPoolSize: BigIntStringString;
  walBufferSize: BigIntStringString;
  dataDirectory: string;
  logLevel: string;
  sslEnabled: boolean;
  uptimeSeconds: BigIntStringString;
  startTime: DateTime;
}

export interface User {
  id: string;
  username: string;
  email?: string | null;
  roles: string[];
  isAdmin: boolean;
  isActive: boolean;
  lastLogin?: DateTime | null;
  createdAt: DateTime;
}

export interface Role {
  id: string;
  name: string;
  description?: string | null;
  permissions: string[];
  isSystem: boolean;
  createdAt: DateTime;
}

export interface HealthStatus {
  status: string;
  components: ComponentHealth[];
  errors: string[];
  warnings: string[];
  checkedAt: DateTime;
}

export interface ComponentHealth {
  name: string;
  status: string;
  responseTimeMs: number;
  details?: string | null;
}

// ============================================================================
// CONNECTION POOL TYPES
// ============================================================================

export interface ConnectionPool {
  id: string;
  name: string;
  minConnections: number;
  maxConnections: number;
  activeConnections: number;
  idleConnections: number;
  totalConnections: number;
  waitingRequests: number;
  connectionTimeoutSeconds: number;
  idleTimeoutSeconds: number;
}

export interface PoolStats {
  poolId: string;
  connectionsCreated: BigIntStringString;
  connectionsDestroyed: BigIntStringString;
  connectionsAcquired: BigIntStringString;
  connectionsReleased: BigIntStringString;
  acquireSuccesses: BigIntStringString;
  acquireFailures: BigIntStringString;
  acquireTimeouts: BigIntStringString;
  avgAcquireTimeMs: number;
  validationFailures: BigIntStringString;
  creationFailures: BigIntStringString;
  leaksDetected: BigIntStringString;
  timestamp: DateTime;
}

export interface Connection {
  id: string;
  sessionId?: string | null;
  username: string;
  clientAddress: string;
  database: string;
  state: string;
  connectedAt: DateTime;
  lastActivity: DateTime;
  currentQuery?: string | null;
  queriesExecuted: BigIntStringString;
  transactionsCount: BigIntStringString;
}

export interface Session {
  id: string;
  userId: string;
  username: string;
  clientAddress: string;
  database: string;
  state: string;
  startedAt: DateTime;
  lastCommand?: string | null;
  lastCommandAt?: DateTime | null;
  queriesExecuted: BigIntStringString;
  idleSeconds: BigIntStringString;
}

export interface Partition {
  id: string;
  name: string;
  tableName: string;
  partitionType: string;
  partitionKey: string;
  partitionValue: string;
  rowCount: BigIntStringString;
  sizeBytes: BigIntStringString;
  isDefault: boolean;
  createdAt: DateTime;
}

// ============================================================================
// ALERT TYPES
// ============================================================================

export interface Alert {
  id: string;
  name: string;
  category: string;
  severity: AlertSeverity;
  state: string;
  message: string;
  details?: string | null;
  triggeredAt: DateTime;
  acknowledgedAt?: DateTime | null;
  resolvedAt?: DateTime | null;
  acknowledgedBy?: string | null;
  escalationLevel: number;
  occurrenceCount: BigIntStringString;
}

export interface ServerInfo {
  version: string;
  buildDate: string;
  gitCommit?: string | null;
  uptimeSeconds: BigIntStringString;
  startTime: DateTime;
  hostname: string;
  os: string;
  arch: string;
  cpuCores: number;
  totalMemory: BigIntStringString;
  pageSize: number;
}
