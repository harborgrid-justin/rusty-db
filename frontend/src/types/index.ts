// ============================================================================
// RustyDB Frontend Type Definitions
// Enterprise-grade type system for the management platform
// ============================================================================

// ============================================================================
// Common Types
// ============================================================================

export type UUID = string;
export type Timestamp = string; // ISO 8601 format
export type Duration = number; // milliseconds

export interface PaginationParams {
  page: number;
  pageSize: number;
  sortBy?: string;
  sortOrder?: 'asc' | 'desc';
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
  hasNext: boolean;
  hasPrevious: boolean;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: ApiError;
  metadata?: Record<string, unknown>;
}

export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  timestamp: Timestamp;
  requestId?: string;
}

// ============================================================================
// Authentication & Authorization Types
// ============================================================================

export interface User {
  id: UUID;
  username: string;
  email?: string;
  displayName?: string;
  roles: Role[];
  permissions: Permission[];
  lastLogin?: Timestamp;
  createdAt: Timestamp;
  updatedAt: Timestamp;
  status: UserStatus;
  settings: UserSettings;
}

export type UserStatus = 'active' | 'inactive' | 'locked' | 'pending';

export interface UserSettings {
  theme: 'light' | 'dark' | 'system';
  timezone: string;
  language: string;
  notifications: NotificationSettings;
}

export interface NotificationSettings {
  email: boolean;
  browser: boolean;
  alertsOnly: boolean;
}

export interface Role {
  id: UUID;
  name: string;
  description?: string;
  permissions: Permission[];
  isSystem: boolean;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface Permission {
  id: UUID;
  name: string;
  resource: string;
  action: PermissionAction;
  description?: string;
}

export type PermissionAction =
  | 'create'
  | 'read'
  | 'update'
  | 'delete'
  | 'execute'
  | 'grant'
  | 'admin';

export interface Session {
  id: UUID;
  userId: UUID;
  token: string;
  refreshToken?: string;
  expiresAt: Timestamp;
  createdAt: Timestamp;
  lastActivity: Timestamp;
  ipAddress: string;
  userAgent: string;
}

export interface LoginCredentials {
  username: string;
  password: string;
  rememberMe?: boolean;
}

export interface AuthState {
  user: User | null;
  session: Session | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
}

// ============================================================================
// Database Schema Types
// ============================================================================

export interface Database {
  name: string;
  owner: string;
  encoding: string;
  collation: string;
  size: number;
  tableCount: number;
  createdAt: Timestamp;
}

export interface Table {
  name: string;
  schema: string;
  columns: Column[];
  primaryKey?: PrimaryKey;
  foreignKeys: ForeignKey[];
  indexes: Index[];
  constraints: Constraint[];
  rowCount: number;
  size: number;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface Column {
  name: string;
  dataType: DataType;
  nullable: boolean;
  defaultValue?: string;
  isPrimaryKey: boolean;
  isForeignKey: boolean;
  isUnique: boolean;
  isIndexed: boolean;
  comment?: string;
  ordinalPosition: number;
}

export type DataType =
  | 'integer'
  | 'bigint'
  | 'smallint'
  | 'decimal'
  | 'numeric'
  | 'real'
  | 'double'
  | 'varchar'
  | 'char'
  | 'text'
  | 'boolean'
  | 'date'
  | 'time'
  | 'timestamp'
  | 'timestamptz'
  | 'interval'
  | 'uuid'
  | 'json'
  | 'jsonb'
  | 'bytea'
  | 'array'
  | 'custom';

export interface PrimaryKey {
  name: string;
  columns: string[];
}

export interface ForeignKey {
  name: string;
  columns: string[];
  referencedTable: string;
  referencedColumns: string[];
  onDelete: ForeignKeyAction;
  onUpdate: ForeignKeyAction;
}

export type ForeignKeyAction = 'cascade' | 'restrict' | 'set_null' | 'set_default' | 'no_action';

export interface Index {
  name: string;
  columns: string[];
  type: IndexType;
  isUnique: boolean;
  isPrimary: boolean;
  size: number;
  usage: IndexUsageStats;
}

export type IndexType = 'btree' | 'hash' | 'gist' | 'gin' | 'brin' | 'spgist' | 'fulltext' | 'spatial';

export interface IndexUsageStats {
  scans: number;
  tuplesRead: number;
  tuplesFetched: number;
  lastUsed?: Timestamp;
}

export interface Constraint {
  name: string;
  type: ConstraintType;
  columns: string[];
  expression?: string;
  isDeferred: boolean;
}

export type ConstraintType = 'primary_key' | 'foreign_key' | 'unique' | 'check' | 'not_null' | 'exclusion';

export interface View {
  name: string;
  schema: string;
  definition: string;
  isMaterialized: boolean;
  columns: Column[];
  dependencies: string[];
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface MaterializedView extends View {
  isMaterialized: true;
  lastRefreshed?: Timestamp;
  refreshSchedule?: string;
  size: number;
}

// ============================================================================
// Query Execution Types
// ============================================================================

export interface QueryRequest {
  sql: string;
  params?: QueryParam[];
  timeout?: number;
  maxRows?: number;
  explain?: boolean;
  analyze?: boolean;
}

export interface QueryParam {
  name: string;
  value: unknown;
  type?: DataType;
}

export interface QueryResult {
  id: UUID;
  sql: string;
  columns: ResultColumn[];
  rows: Record<string, unknown>[];
  rowCount: number;
  affectedRows?: number;
  executionTime: Duration;
  planningTime?: Duration;
  status: QueryStatus;
  warnings?: string[];
  explain?: ExplainPlan;
}

export interface ResultColumn {
  name: string;
  type: DataType;
  nullable: boolean;
}

export type QueryStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface ExplainPlan {
  plan: PlanNode;
  executionTime?: Duration;
  planningTime?: Duration;
  totalCost: number;
  triggers?: TriggerPlan[];
}

export interface PlanNode {
  nodeType: string;
  relationName?: string;
  alias?: string;
  startupCost: number;
  totalCost: number;
  planRows: number;
  planWidth: number;
  actualStartupTime?: number;
  actualTotalTime?: number;
  actualRows?: number;
  actualLoops?: number;
  filter?: string;
  rowsRemovedByFilter?: number;
  indexName?: string;
  indexCond?: string;
  joinType?: string;
  hashCond?: string;
  sortKey?: string[];
  children?: PlanNode[];
}

export interface TriggerPlan {
  name: string;
  relation: string;
  time: number;
  calls: number;
}

export interface QueryHistory {
  id: UUID;
  sql: string;
  executedAt: Timestamp;
  executionTime: Duration;
  status: QueryStatus;
  rowCount?: number;
  userId: UUID;
  database: string;
}

export interface SavedQuery {
  id: UUID;
  name: string;
  description?: string;
  sql: string;
  params?: QueryParam[];
  tags: string[];
  isPublic: boolean;
  createdBy: UUID;
  createdAt: Timestamp;
  updatedAt: Timestamp;
  lastExecuted?: Timestamp;
  executionCount: number;
}

// ============================================================================
// Monitoring & Metrics Types
// ============================================================================

export interface SystemMetrics {
  timestamp: Timestamp;
  cpu: CpuMetrics;
  memory: MemoryMetrics;
  disk: DiskMetrics;
  network: NetworkMetrics;
  database: DatabaseMetrics;
}

export interface CpuMetrics {
  usage: number; // percentage
  userTime: number;
  systemTime: number;
  idleTime: number;
  loadAverage: [number, number, number]; // 1, 5, 15 min
  cores: number;
}

export interface MemoryMetrics {
  total: number; // bytes
  used: number;
  free: number;
  cached: number;
  buffers: number;
  swapTotal: number;
  swapUsed: number;
  usagePercent: number;
}

export interface DiskMetrics {
  total: number; // bytes
  used: number;
  free: number;
  usagePercent: number;
  readOps: number;
  writeOps: number;
  readBytes: number;
  writeBytes: number;
  avgReadLatency: Duration;
  avgWriteLatency: Duration;
}

export interface NetworkMetrics {
  bytesReceived: number;
  bytesSent: number;
  packetsReceived: number;
  packetsSent: number;
  errors: number;
  dropped: number;
  activeConnections: number;
}

export interface DatabaseMetrics {
  activeConnections: number;
  idleConnections: number;
  maxConnections: number;
  transactionsPerSecond: number;
  queriesPerSecond: number;
  cacheHitRatio: number;
  bufferPoolUsage: number;
  lockWaits: number;
  deadlocks: number;
  replicationLag?: Duration;
}

export interface QueryMetrics {
  totalQueries: number;
  selectQueries: number;
  insertQueries: number;
  updateQueries: number;
  deleteQueries: number;
  avgExecutionTime: Duration;
  slowQueries: number;
  failedQueries: number;
  activeQueries: number;
}

export interface ConnectionPoolStats {
  poolId: string;
  minConnections: number;
  maxConnections: number;
  activeConnections: number;
  idleConnections: number;
  waitingRequests: number;
  totalConnections: number;
  avgWaitTime: Duration;
  avgConnectionTime: Duration;
}

export interface ActiveSession {
  id: UUID;
  userId?: string;
  database: string;
  clientAddress: string;
  clientPort: number;
  backendStart: Timestamp;
  state: SessionState;
  currentQuery?: string;
  queryStart?: Timestamp;
  waitEvent?: string;
  waitEventType?: string;
  blockedBy?: UUID;
}

export type SessionState = 'active' | 'idle' | 'idle_in_transaction' | 'idle_in_transaction_aborted' | 'fastpath' | 'disabled';

export interface SlowQuery {
  id: UUID;
  sql: string;
  executionTime: Duration;
  userId?: string;
  database: string;
  timestamp: Timestamp;
  rowsAffected?: number;
  explain?: ExplainPlan;
}

export interface HealthStatus {
  status: HealthState;
  components: ComponentHealth[];
  timestamp: Timestamp;
  uptime: Duration;
  version: string;
}

export type HealthState = 'healthy' | 'degraded' | 'unhealthy' | 'critical';

export interface ComponentHealth {
  name: string;
  status: HealthState;
  message?: string;
  lastCheck: Timestamp;
  responseTime?: Duration;
  details?: Record<string, unknown>;
}

export interface Alert {
  id: UUID;
  type: AlertType;
  severity: AlertSeverity;
  title: string;
  message: string;
  source: string;
  timestamp: Timestamp;
  acknowledged: boolean;
  acknowledgedBy?: string;
  acknowledgedAt?: Timestamp;
  resolved: boolean;
  resolvedAt?: Timestamp;
  metadata?: Record<string, unknown>;
}

export type AlertType =
  | 'performance'
  | 'security'
  | 'availability'
  | 'capacity'
  | 'replication'
  | 'backup'
  | 'configuration';

export type AlertSeverity = 'info' | 'warning' | 'error' | 'critical';

// ============================================================================
// Security Types
// ============================================================================

export interface EncryptionKey {
  id: UUID;
  name: string;
  algorithm: EncryptionAlgorithm;
  keyType: KeyType;
  status: KeyStatus;
  createdAt: Timestamp;
  expiresAt?: Timestamp;
  rotatedAt?: Timestamp;
  version: number;
  metadata?: Record<string, unknown>;
}

export type EncryptionAlgorithm = 'AES256GCM' | 'ChaCha20Poly1305' | 'RSA4096';
export type KeyType = 'master' | 'data' | 'backup' | 'transport';
export type KeyStatus = 'active' | 'inactive' | 'expired' | 'compromised' | 'pending_rotation';

export interface DataMaskingPolicy {
  id: UUID;
  name: string;
  description?: string;
  table: string;
  column: string;
  maskingType: MaskingType;
  maskingFunction?: string;
  applyTo: string[]; // roles or users
  isEnabled: boolean;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export type MaskingType =
  | 'full'
  | 'partial'
  | 'email'
  | 'phone'
  | 'ssn'
  | 'credit_card'
  | 'custom'
  | 'hash'
  | 'null';

export interface AuditLog {
  id: UUID;
  timestamp: Timestamp;
  eventType: AuditEventType;
  userId?: string;
  username?: string;
  clientAddress?: string;
  database?: string;
  objectType?: string;
  objectName?: string;
  action: string;
  sqlText?: string;
  oldValue?: unknown;
  newValue?: unknown;
  status: 'success' | 'failure';
  errorMessage?: string;
  metadata?: Record<string, unknown>;
}

export type AuditEventType =
  | 'authentication'
  | 'authorization'
  | 'ddl'
  | 'dml'
  | 'dcl'
  | 'configuration'
  | 'security'
  | 'system';

export interface SecurityPolicy {
  id: UUID;
  name: string;
  type: SecurityPolicyType;
  table: string;
  expression: string;
  roles: string[];
  isEnabled: boolean;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export type SecurityPolicyType = 'row_level' | 'column_level' | 'vpd';

// ============================================================================
// Backup & Recovery Types
// ============================================================================

export interface Backup {
  id: UUID;
  name: string;
  type: BackupType;
  status: BackupStatus;
  database?: string;
  size: number;
  compressedSize?: number;
  startTime: Timestamp;
  endTime?: Timestamp;
  duration?: Duration;
  location: string;
  checksum?: string;
  encrypted: boolean;
  retentionDays: number;
  expiresAt?: Timestamp;
  metadata?: Record<string, unknown>;
}

export type BackupType = 'full' | 'incremental' | 'differential' | 'logical' | 'physical';
export type BackupStatus = 'pending' | 'running' | 'completed' | 'failed' | 'expired' | 'deleted';

export interface BackupSchedule {
  id: UUID;
  name: string;
  type: BackupType;
  database?: string;
  schedule: string; // cron expression
  retentionDays: number;
  isEnabled: boolean;
  lastRun?: Timestamp;
  nextRun?: Timestamp;
  lastStatus?: BackupStatus;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface RestoreRequest {
  backupId: UUID;
  targetDatabase?: string;
  pointInTime?: Timestamp;
  options?: RestoreOptions;
}

export interface RestoreOptions {
  createDatabase: boolean;
  dropExisting: boolean;
  parallel: number;
  includeIndexes: boolean;
  includeConstraints: boolean;
}

export interface RestoreProgress {
  id: UUID;
  backupId: UUID;
  status: RestoreStatus;
  progress: number; // percentage
  currentPhase: string;
  startTime: Timestamp;
  estimatedCompletion?: Timestamp;
  bytesRestored: number;
  totalBytes: number;
  tablesRestored: number;
  totalTables: number;
  errors: string[];
}

export type RestoreStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

// ============================================================================
// Cluster & Replication Types
// ============================================================================

export interface ClusterNode {
  id: UUID;
  name: string;
  host: string;
  port: number;
  role: NodeRole;
  status: NodeStatus;
  region?: string;
  zone?: string;
  version: string;
  startTime: Timestamp;
  lastHeartbeat: Timestamp;
  metrics?: NodeMetrics;
}

export type NodeRole = 'leader' | 'follower' | 'candidate' | 'observer';
export type NodeStatus = 'healthy' | 'degraded' | 'unreachable' | 'shutting_down' | 'failed';

export interface NodeMetrics {
  cpu: number;
  memory: number;
  disk: number;
  connections: number;
  replicationLag?: Duration;
  queriesPerSecond: number;
}

export interface ClusterTopology {
  nodes: ClusterNode[];
  leader?: UUID;
  term: number;
  configVersion: number;
  lastUpdated: Timestamp;
}

export interface ReplicationStatus {
  sourceNode: UUID;
  targetNode: UUID;
  status: ReplicationState;
  mode: ReplicationMode;
  lag: Duration;
  lastSyncedLsn: string;
  lastSyncTime: Timestamp;
  bytesTransferred: number;
  transactionsReplicated: number;
}

export type ReplicationState = 'streaming' | 'catchup' | 'stopped' | 'error';
export type ReplicationMode = 'synchronous' | 'asynchronous' | 'semi_synchronous';

export interface FailoverEvent {
  id: UUID;
  timestamp: Timestamp;
  type: FailoverType;
  oldLeader: UUID;
  newLeader: UUID;
  reason: string;
  duration: Duration;
  status: 'success' | 'failed';
  details?: Record<string, unknown>;
}

export type FailoverType = 'automatic' | 'manual' | 'planned';

// ============================================================================
// Configuration Types
// ============================================================================

export interface DatabaseConfig {
  general: GeneralConfig;
  performance: PerformanceConfig;
  security: SecurityConfig;
  logging: LoggingConfig;
  replication: ReplicationConfig;
  maintenance: MaintenanceConfig;
}

export interface GeneralConfig {
  dataDirectory: string;
  listenAddress: string;
  port: number;
  maxConnections: number;
  timezone: string;
  encoding: string;
}

export interface PerformanceConfig {
  sharedBuffers: number;
  workMem: number;
  maintenanceWorkMem: number;
  effectiveCacheSize: number;
  walBuffers: number;
  checkpointSegments: number;
  randomPageCost: number;
  effectiveIoConcurrency: number;
  parallelWorkers: number;
}

export interface SecurityConfig {
  authenticationEnabled: boolean;
  encryptionEnabled: boolean;
  sslEnabled: boolean;
  sslCertFile?: string;
  sslKeyFile?: string;
  passwordPolicy: PasswordPolicy;
  sessionTimeout: Duration;
  maxFailedLogins: number;
}

export interface PasswordPolicy {
  minLength: number;
  requireUppercase: boolean;
  requireLowercase: boolean;
  requireNumbers: boolean;
  requireSpecialChars: boolean;
  maxAge: number; // days
  historyCount: number;
}

export interface LoggingConfig {
  level: LogLevel;
  destination: LogDestination;
  filePath?: string;
  maxFileSize: number;
  maxFiles: number;
  includeTimestamp: boolean;
  includeSourceLocation: boolean;
  slowQueryThreshold: Duration;
}

export type LogLevel = 'debug' | 'info' | 'warning' | 'error';
export type LogDestination = 'console' | 'file' | 'syslog' | 'both';

export interface ReplicationConfig {
  enabled: boolean;
  mode: ReplicationMode;
  syncStandbyNames?: string[];
  walLevel: 'minimal' | 'replica' | 'logical';
  maxWalSenders: number;
  walKeepSegments: number;
}

export interface MaintenanceConfig {
  autoVacuum: boolean;
  vacuumThreshold: number;
  analyzeThreshold: number;
  autoVacuumWorkers: number;
  maintenanceWindow?: string;
}

// ============================================================================
// Resource Management Types
// ============================================================================

export interface ResourceGroup {
  id: UUID;
  name: string;
  cpuLimit: number; // percentage
  memoryLimit: number; // bytes
  ioLimit?: number; // IOPS
  maxConnections: number;
  maxQueries: number;
  queryTimeout: Duration;
  priority: number;
  members: string[]; // user or role names
  isEnabled: boolean;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface ResourceUsage {
  groupId: UUID;
  timestamp: Timestamp;
  cpuUsage: number;
  memoryUsage: number;
  ioUsage: number;
  activeConnections: number;
  activeQueries: number;
  queuedQueries: number;
}

// ============================================================================
// UI State Types
// ============================================================================

export interface SidebarState {
  isOpen: boolean;
  isPinned: boolean;
  activeSection: string;
}

export interface TabState {
  id: UUID;
  type: TabType;
  title: string;
  data?: unknown;
  isDirty: boolean;
}

export type TabType =
  | 'query'
  | 'table'
  | 'view'
  | 'config'
  | 'logs'
  | 'metrics'
  | 'backup'
  | 'cluster';

export interface NotificationItem {
  id: UUID;
  type: 'info' | 'success' | 'warning' | 'error';
  title: string;
  message: string;
  timestamp: Timestamp;
  read: boolean;
  actionUrl?: string;
  actionLabel?: string;
}

export interface ConfirmDialogState {
  isOpen: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  variant: 'danger' | 'warning' | 'info';
  onConfirm: () => void;
  onCancel: () => void;
}

// ============================================================================
// Export all types
// ============================================================================

export type {
  UUID as Id,
  Timestamp as DateTime,
  Duration as Ms,
};
