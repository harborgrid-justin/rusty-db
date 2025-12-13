/**
 * RustyDB Backup & Recovery API Client Tests
 *
 * Comprehensive test suite covering:
 * - Full backup creation and management
 * - Incremental backup creation and management
 * - Restore operations with PITR
 * - Flashback query operations
 * - Flashback table operations
 * - Restore points management
 * - Version queries (time-travel)
 * - Database flashback
 * - Transaction flashback
 * - Backup scheduling
 */

import { describe, it, expect, beforeEach, afterEach, jest } from '@jest/globals';
import MockAdapter from 'axios-mock-adapter';
import axios from 'axios';
import {
  BackupRecoveryClient,
  createBackupRecoveryClient,
  BackupDetails,
  BackupList,
  BackupSchedule,
  RestoreResponse,
  FlashbackQueryResponse,
  FlashbackTableResponse,
  VersionsQueryResponse,
  RestorePointResponse,
  RestorePointInfo,
  FlashbackDatabaseResponse,
  FlashbackStatsResponse,
  TransactionFlashbackResponse,
  RowVersion,
} from '../src/api/backup-recovery';

describe('BackupRecoveryClient', () => {
  let client: BackupRecoveryClient;
  let mock: MockAdapter;

  beforeEach(() => {
    client = createBackupRecoveryClient({
      baseURL: 'http://localhost:5432',
      timeout: 5000,
    });
    mock = new MockAdapter(axios);
  });

  afterEach(() => {
    mock.reset();
  });

  // ==========================================================================
  // Backup Creation Tests
  // ==========================================================================

  describe('Backup Creation', () => {
    it('should create a full backup successfully', async () => {
      const mockBackup: BackupDetails = {
        backup_id: 'backup_123e4567-e89b-12d3-a456-426614174000',
        backup_type: 'full',
        status: 'in_progress',
        database_name: 'rustydb',
        start_time: 1702512000,
        location: '/var/lib/rustydb/backups/backup_123',
        compression_enabled: true,
        encryption_enabled: true,
        retention_until: 1705190400,
        description: 'Monthly full backup',
      };

      mock.onPost('/api/v1/backup/full').reply(202, mockBackup);

      const result = await client.createFullBackup({
        backup_type: 'full',
        compression: true,
        encryption: true,
        retention_days: 30,
        description: 'Monthly full backup',
      });

      expect(result).toEqual(mockBackup);
      expect(result.backup_type).toBe('full');
      expect(result.compression_enabled).toBe(true);
      expect(result.encryption_enabled).toBe(true);
    });

    it('should create an incremental backup successfully', async () => {
      const mockBackup: BackupDetails = {
        backup_id: 'backup_223e4567-e89b-12d3-a456-426614174001',
        backup_type: 'incremental',
        status: 'in_progress',
        database_name: 'rustydb',
        start_time: 1702598400,
        location: '/var/lib/rustydb/backups/backup_223',
        compression_enabled: true,
        encryption_enabled: false,
        description: 'Daily incremental backup',
      };

      mock.onPost('/api/v1/backup/incremental').reply(202, mockBackup);

      const result = await client.createIncrementalBackup({
        backup_type: 'incremental',
        compression: true,
        encryption: false,
        description: 'Daily incremental backup',
      });

      expect(result).toEqual(mockBackup);
      expect(result.backup_type).toBe('incremental');
      expect(result.status).toBe('in_progress');
    });

    it('should create backup using generic method', async () => {
      const mockBackup: BackupDetails = {
        backup_id: 'backup_323e4567',
        backup_type: 'full',
        status: 'in_progress',
        database_name: 'rustydb',
        start_time: 1702684800,
        location: '/var/lib/rustydb/backups/backup_323',
        compression_enabled: false,
        encryption_enabled: false,
      };

      mock.onPost('/api/v1/backup/full').reply(202, mockBackup);

      const result = await client.createBackup('full', {
        compression: false,
        encryption: false,
      });

      expect(result.backup_type).toBe('full');
    });

    it('should create backup with custom destination', async () => {
      const customDestination = '/mnt/backup-storage/rustydb/backup_001';
      const mockBackup: BackupDetails = {
        backup_id: 'backup_001',
        backup_type: 'full',
        status: 'in_progress',
        database_name: 'rustydb',
        start_time: 1702771200,
        location: customDestination,
        compression_enabled: true,
        encryption_enabled: true,
      };

      mock.onPost('/api/v1/backup/full').reply(202, mockBackup);

      const result = await client.createFullBackup({
        backup_type: 'full',
        destination: customDestination,
        compression: true,
        encryption: true,
      });

      expect(result.location).toBe(customDestination);
    });
  });

  // ==========================================================================
  // Backup Management Tests
  // ==========================================================================

  describe('Backup Management', () => {
    it('should list all backups', async () => {
      const mockList: BackupList = {
        backups: [
          {
            backup_id: 'backup_001',
            backup_type: 'full',
            status: 'completed',
            start_time: 1702512000,
            size_bytes: 104857600,
            location: '/var/lib/rustydb/backups/backup_001',
          },
          {
            backup_id: 'backup_002',
            backup_type: 'incremental',
            status: 'completed',
            start_time: 1702598400,
            size_bytes: 10485760,
            location: '/var/lib/rustydb/backups/backup_002',
          },
        ],
        total_count: 2,
      };

      mock.onGet('/api/v1/backup/list').reply(200, mockList);

      const result = await client.listBackups();

      expect(result.total_count).toBe(2);
      expect(result.backups).toHaveLength(2);
      expect(result.backups[0].backup_type).toBe('full');
      expect(result.backups[1].backup_type).toBe('incremental');
    });

    it('should get backup details by ID', async () => {
      const backupId = 'backup_123';
      const mockBackup: BackupDetails = {
        backup_id: backupId,
        backup_type: 'full',
        status: 'completed',
        database_name: 'rustydb',
        start_time: 1702512000,
        completion_time: 1702515600,
        size_bytes: 104857600,
        compressed_size_bytes: 52428800,
        location: '/var/lib/rustydb/backups/backup_123',
        compression_enabled: true,
        encryption_enabled: true,
      };

      mock.onGet(`/api/v1/backup/${backupId}`).reply(200, mockBackup);

      const result = await client.getBackup(backupId);

      expect(result.backup_id).toBe(backupId);
      expect(result.status).toBe('completed');
      expect(result.size_bytes).toBe(104857600);
      expect(result.compressed_size_bytes).toBe(52428800);
    });

    it('should delete a backup', async () => {
      const backupId = 'backup_456';

      mock.onDelete(`/api/v1/backup/${backupId}`).reply(204);

      await expect(client.deleteBackup(backupId)).resolves.not.toThrow();
    });

    it('should handle backup not found error', async () => {
      const backupId = 'nonexistent';

      mock.onGet(`/api/v1/backup/${backupId}`).reply(404, {
        code: 'NOT_FOUND',
        message: `Backup '${backupId}' not found`,
      });

      await expect(client.getBackup(backupId)).rejects.toThrow();
    });
  });

  // ==========================================================================
  // Restore Operations Tests
  // ==========================================================================

  describe('Restore Operations', () => {
    it('should restore from backup', async () => {
      const backupId = 'backup_789';
      const mockRestore: RestoreResponse = {
        restore_id: 'restore_001',
        status: 'in_progress',
        message: `Restore started from backup ${backupId}`,
        started_at: 1702857600,
      };

      mock.onPost(`/api/v1/backup/${backupId}/restore`).reply(202, mockRestore);

      const result = await client.restoreBackup(backupId, {
        target_database: 'rustydb_restored',
        overwrite_existing: false,
      });

      expect(result.restore_id).toBe('restore_001');
      expect(result.status).toBe('in_progress');
    });

    it('should perform point-in-time recovery (PITR)', async () => {
      const backupId = 'backup_full_001';
      const targetTime = new Date('2024-12-15T10:30:00Z');
      const mockRestore: RestoreResponse = {
        restore_id: 'restore_pitr_001',
        status: 'in_progress',
        message: 'Point-in-time restore started',
        started_at: 1702944000,
      };

      mock.onPost(`/api/v1/backup/${backupId}/restore`).reply(202, mockRestore);

      const result = await client.restoreToPointInTime(backupId, targetTime, {
        targetDatabase: 'rustydb_pitr',
        verifyOnly: false,
      });

      expect(result.restore_id).toBe('restore_pitr_001');
      expect(result.status).toBe('in_progress');
    });

    it('should verify backup integrity', async () => {
      const backupId = 'backup_verify_001';
      const mockRestore: RestoreResponse = {
        restore_id: 'verify_001',
        status: 'in_progress',
        message: `Backup verification started for backup ${backupId}`,
        started_at: 1703030400,
      };

      mock.onPost(`/api/v1/backup/${backupId}/restore`).reply(202, mockRestore);

      const result = await client.verifyBackup(backupId);

      expect(result.message).toContain('verification');
    });

    it('should handle restore request with all options', async () => {
      const backupId = 'backup_complete';
      const mockRestore: RestoreResponse = {
        restore_id: 'restore_complete_001',
        status: 'in_progress',
        message: 'Restore started',
        started_at: 1703116800,
      };

      mock.onPost(`/api/v1/backup/${backupId}/restore`).reply(202, mockRestore);

      const result = await client.restoreBackup(backupId, {
        target_database: 'test_db',
        point_in_time: 1703000000,
        verify_only: false,
        overwrite_existing: true,
      });

      expect(result.status).toBe('in_progress');
    });
  });

  // ==========================================================================
  // Backup Schedule Tests
  // ==========================================================================

  describe('Backup Schedule', () => {
    it('should get backup schedule', async () => {
      const mockSchedule: BackupSchedule = {
        enabled: true,
        full_backup_cron: '0 2 * * 0',
        incremental_backup_cron: '0 2 * * 1-6',
        retention_days: 30,
        compression: true,
        encryption: true,
        destination: '/var/lib/rustydb/backups',
      };

      mock.onGet('/api/v1/backup/schedule').reply(200, mockSchedule);

      const result = await client.getBackupSchedule();

      expect(result.enabled).toBe(true);
      expect(result.full_backup_cron).toBe('0 2 * * 0');
      expect(result.incremental_backup_cron).toBe('0 2 * * 1-6');
      expect(result.retention_days).toBe(30);
    });

    it('should update backup schedule', async () => {
      const newSchedule: BackupSchedule = {
        enabled: true,
        full_backup_cron: '0 3 * * 0',
        incremental_backup_cron: '0 3 * * 1-6',
        retention_days: 60,
        compression: true,
        encryption: true,
        destination: '/mnt/backups',
      };

      const mockResponse = {
        success: true,
        message: 'Backup schedule updated successfully',
        enabled: true,
      };

      mock.onPut('/api/v1/backup/schedule').reply(200, mockResponse);

      const result = await client.updateBackupSchedule(newSchedule);

      expect(result.success).toBe(true);
      expect(result.enabled).toBe(true);
    });

    it('should enable backup schedule', async () => {
      const currentSchedule: BackupSchedule = {
        enabled: false,
        full_backup_cron: '0 2 * * 0',
        incremental_backup_cron: '0 2 * * 1-6',
        retention_days: 30,
        compression: true,
        encryption: true,
        destination: '/var/lib/rustydb/backups',
      };

      mock.onGet('/api/v1/backup/schedule').reply(200, currentSchedule);
      mock.onPut('/api/v1/backup/schedule').reply(200, {
        success: true,
        message: 'Backup schedule updated',
        enabled: true,
      });

      await client.enableBackupSchedule({ retention_days: 45 });

      expect(mock.history.put.length).toBe(1);
    });

    it('should disable backup schedule', async () => {
      const currentSchedule: BackupSchedule = {
        enabled: true,
        full_backup_cron: '0 2 * * 0',
        incremental_backup_cron: '0 2 * * 1-6',
        retention_days: 30,
        compression: true,
        encryption: true,
        destination: '/var/lib/rustydb/backups',
      };

      mock.onGet('/api/v1/backup/schedule').reply(200, currentSchedule);
      mock.onPut('/api/v1/backup/schedule').reply(200, {
        success: true,
        message: 'Backup schedule disabled',
        enabled: false,
      });

      await client.disableBackupSchedule();

      expect(mock.history.put.length).toBe(1);
    });
  });

  // ==========================================================================
  // Flashback Query Tests
  // ==========================================================================

  describe('Flashback Query', () => {
    it('should execute flashback query with timestamp', async () => {
      const mockResponse: FlashbackQueryResponse = {
        rows: [
          { id: 1, name: 'Alice', balance: 1000 },
          { id: 2, name: 'Bob', balance: 2000 },
        ],
        count: 2,
        query_scn: 12345,
        query_timestamp: 1703203200,
      };

      mock.onPost('/api/v1/flashback/query').reply(200, mockResponse);

      const result = await client.flashbackQuery({
        table: 'accounts',
        timestamp: '2024-12-15T10:00:00Z',
        columns: ['id', 'name', 'balance'],
      });

      expect(result.count).toBe(2);
      expect(result.rows).toHaveLength(2);
      expect(result.query_scn).toBe(12345);
    });

    it('should query as of timestamp', async () => {
      const mockResponse: FlashbackQueryResponse = {
        rows: [{ id: 1, value: 'old_value' }],
        count: 1,
        query_scn: 10000,
        query_timestamp: 1703203200,
      };

      mock.onPost('/api/v1/flashback/query').reply(200, mockResponse);

      const timestamp = new Date('2024-12-10T00:00:00Z');
      const result = await client.queryAsOfTimestamp('users', timestamp, {
        filter: { active: true },
        limit: 100,
      });

      expect(result.rows).toHaveLength(1);
    });

    it('should query as of SCN', async () => {
      const mockResponse: FlashbackQueryResponse = {
        rows: [{ id: 5, data: 'historical_data' }],
        count: 1,
        query_scn: 50000,
        query_timestamp: 1703203200,
      };

      mock.onPost('/api/v1/flashback/query').reply(200, mockResponse);

      const result = await client.queryAsOfSCN('transactions', 50000, {
        columns: ['id', 'data'],
      });

      expect(result.query_scn).toBe(50000);
    });

    it('should handle flashback query with filter', async () => {
      const mockResponse: FlashbackQueryResponse = {
        rows: [{ id: 10, status: 'active' }],
        count: 1,
        query_scn: 60000,
        query_timestamp: 1703289600,
      };

      mock.onPost('/api/v1/flashback/query').reply(200, mockResponse);

      const result = await client.flashbackQuery({
        table: 'orders',
        scn: 60000,
        filter: { status: 'active', amount: { $gt: 1000 } },
        limit: 50,
      });

      expect(result.count).toBe(1);
    });
  });

  // ==========================================================================
  // Flashback Table Tests
  // ==========================================================================

  describe('Flashback Table', () => {
    it('should restore table to timestamp', async () => {
      const mockResponse: FlashbackTableResponse = {
        table: 'products',
        status: 'restored',
        rows_restored: 150,
        restore_timestamp: 1703376000,
        duration_ms: 5000,
      };

      mock.onPost('/api/v1/flashback/table').reply(200, mockResponse);

      const timestamp = new Date('2024-12-14T12:00:00Z');
      const result = await client.restoreTableToTimestamp('products', timestamp, {
        enableTriggers: true,
      });

      expect(result.table).toBe('products');
      expect(result.rows_restored).toBe(150);
      expect(result.status).toBe('restored');
    });

    it('should restore table to SCN', async () => {
      const mockResponse: FlashbackTableResponse = {
        table: 'inventory',
        status: 'restored',
        rows_restored: 500,
        restore_timestamp: 1703462400,
        duration_ms: 10000,
      };

      mock.onPost('/api/v1/flashback/table').reply(200, mockResponse);

      const result = await client.restoreTableToSCN('inventory', 75000);

      expect(result.table).toBe('inventory');
      expect(result.rows_restored).toBe(500);
    });

    it('should restore table to restore point', async () => {
      const mockResponse: FlashbackTableResponse = {
        table: 'customers',
        status: 'restored',
        rows_restored: 1000,
        restore_timestamp: 1703548800,
        duration_ms: 15000,
      };

      mock.onPost('/api/v1/flashback/table').reply(200, mockResponse);

      const result = await client.restoreTableToRestorePoint(
        'customers',
        'before_migration'
      );

      expect(result.table).toBe('customers');
      expect(result.status).toBe('restored');
    });

    it('should handle table flashback with all options', async () => {
      const mockResponse: FlashbackTableResponse = {
        table: 'orders',
        status: 'restored',
        rows_restored: 250,
        restore_timestamp: 1703635200,
        duration_ms: 7500,
      };

      mock.onPost('/api/v1/flashback/table').reply(200, mockResponse);

      const result = await client.flashbackTable({
        table: 'orders',
        target_timestamp: '2024-12-16T08:00:00Z',
        enable_triggers: false,
      });

      expect(result.rows_restored).toBe(250);
    });
  });

  // ==========================================================================
  // Version Query Tests (Row History)
  // ==========================================================================

  describe('Version Queries', () => {
    it('should query row versions', async () => {
      const mockVersions: RowVersion[] = [
        {
          scn: 10000,
          timestamp: 1703116800,
          operation: 'INSERT',
          transaction_id: 'txn_001',
          data: { id: 1, balance: 1000 },
        },
        {
          scn: 20000,
          timestamp: 1703203200,
          operation: 'UPDATE',
          transaction_id: 'txn_002',
          data: { id: 1, balance: 1500 },
          changed_columns: ['balance'],
        },
        {
          scn: 30000,
          timestamp: 1703289600,
          operation: 'UPDATE',
          transaction_id: 'txn_003',
          data: { id: 1, balance: 2000 },
          changed_columns: ['balance'],
        },
      ];

      const mockResponse: VersionsQueryResponse = {
        versions: mockVersions,
        count: 3,
      };

      mock.onPost('/api/v1/flashback/versions').reply(200, mockResponse);

      const result = await client.queryVersions({
        table: 'accounts',
        primary_key: { id: 1 },
        start_scn: 10000,
        end_scn: 30000,
      });

      expect(result.count).toBe(3);
      expect(result.versions).toHaveLength(3);
      expect(result.versions[0].operation).toBe('INSERT');
      expect(result.versions[1].operation).toBe('UPDATE');
    });

    it('should get full row history', async () => {
      const mockVersions: RowVersion[] = [
        {
          scn: 5000,
          timestamp: 1703030400,
          operation: 'INSERT',
          transaction_id: 'txn_100',
          data: { user_id: 42, email: 'old@example.com' },
        },
        {
          scn: 15000,
          timestamp: 1703376000,
          operation: 'UPDATE',
          transaction_id: 'txn_200',
          data: { user_id: 42, email: 'new@example.com' },
          changed_columns: ['email'],
        },
      ];

      mock.onPost('/api/v1/flashback/versions').reply(200, {
        versions: mockVersions,
        count: 2,
      });

      const history = await client.getRowHistory('users', { user_id: 42 });

      expect(history).toHaveLength(2);
      expect(history[0].data.email).toBe('old@example.com');
      expect(history[1].data.email).toBe('new@example.com');
    });

    it('should query versions with timestamp range', async () => {
      const mockResponse: VersionsQueryResponse = {
        versions: [],
        count: 0,
      };

      mock.onPost('/api/v1/flashback/versions').reply(200, mockResponse);

      const result = await client.getRowHistory(
        'products',
        { product_id: 100 },
        {
          startTimestamp: '2024-12-01T00:00:00Z',
          endTimestamp: '2024-12-31T23:59:59Z',
        }
      );

      expect(result).toHaveLength(0);
    });
  });

  // ==========================================================================
  // Restore Points Tests
  // ==========================================================================

  describe('Restore Points', () => {
    it('should create a restore point', async () => {
      const mockResponse: RestorePointResponse = {
        name: 'before_upgrade',
        scn: 100000,
        timestamp: 1703721600,
        guaranteed: false,
      };

      mock.onPost('/api/v1/flashback/restore-points').reply(201, mockResponse);

      const result = await client.createRestorePoint({
        name: 'before_upgrade',
        guaranteed: false,
      });

      expect(result.name).toBe('before_upgrade');
      expect(result.scn).toBe(100000);
      expect(result.guaranteed).toBe(false);
    });

    it('should create a guaranteed restore point', async () => {
      const mockResponse: RestorePointResponse = {
        name: 'critical_checkpoint',
        scn: 150000,
        timestamp: 1703808000,
        guaranteed: true,
      };

      mock.onPost('/api/v1/flashback/restore-points').reply(201, mockResponse);

      const result = await client.createGuaranteedRestorePoint(
        'critical_checkpoint',
        true
      );

      expect(result.guaranteed).toBe(true);
      expect(result.name).toBe('critical_checkpoint');
    });

    it('should create a normal restore point', async () => {
      const mockResponse: RestorePointResponse = {
        name: 'daily_checkpoint',
        scn: 125000,
        timestamp: 1703894400,
        guaranteed: false,
      };

      mock.onPost('/api/v1/flashback/restore-points').reply(201, mockResponse);

      const result = await client.createNormalRestorePoint('daily_checkpoint');

      expect(result.guaranteed).toBe(false);
    });

    it('should list restore points', async () => {
      const mockRestorePoints: RestorePointInfo[] = [
        {
          name: 'checkpoint_1',
          scn: 100000,
          timestamp: 1703721600,
          guaranteed: false,
        },
        {
          name: 'checkpoint_2',
          scn: 150000,
          timestamp: 1703808000,
          guaranteed: true,
        },
      ];

      mock.onGet('/api/v1/flashback/restore-points').reply(200, mockRestorePoints);

      const result = await client.listRestorePoints();

      expect(result).toHaveLength(2);
      expect(result[0].name).toBe('checkpoint_1');
      expect(result[1].guaranteed).toBe(true);
    });

    it('should delete a restore point', async () => {
      const pointName = 'old_checkpoint';

      mock.onDelete(`/api/v1/flashback/restore-points/${pointName}`).reply(204);

      await expect(client.deleteRestorePoint(pointName)).resolves.not.toThrow();
    });
  });

  // ==========================================================================
  // Database Flashback Tests
  // ==========================================================================

  describe('Database Flashback', () => {
    it('should flashback database to timestamp', async () => {
      const mockResponse: FlashbackDatabaseResponse = {
        status: 'completed',
        target_scn: 200000,
        target_timestamp: 1703980800,
        duration_ms: 60000,
      };

      mock.onPost('/api/v1/flashback/database').reply(200, mockResponse);

      const timestamp = new Date('2024-12-20T12:00:00Z');
      const result = await client.flashbackDatabaseToTimestamp(timestamp);

      expect(result.status).toBe('completed');
      expect(result.target_scn).toBe(200000);
      expect(result.duration_ms).toBe(60000);
    });

    it('should flashback database to SCN', async () => {
      const mockResponse: FlashbackDatabaseResponse = {
        status: 'completed',
        target_scn: 180000,
        target_timestamp: 1704067200,
        duration_ms: 45000,
      };

      mock.onPost('/api/v1/flashback/database').reply(200, mockResponse);

      const result = await client.flashbackDatabaseToSCN(180000);

      expect(result.target_scn).toBe(180000);
    });

    it('should flashback database to restore point', async () => {
      const mockResponse: FlashbackDatabaseResponse = {
        status: 'completed',
        target_scn: 175000,
        target_timestamp: 1704153600,
        duration_ms: 50000,
      };

      mock.onPost('/api/v1/flashback/database').reply(200, mockResponse);

      const result = await client.flashbackDatabaseToRestorePoint('pre_migration');

      expect(result.status).toBe('completed');
    });

    it('should handle database flashback with all options', async () => {
      const mockResponse: FlashbackDatabaseResponse = {
        status: 'completed',
        target_scn: 190000,
        target_timestamp: 1704240000,
        duration_ms: 55000,
      };

      mock.onPost('/api/v1/flashback/database').reply(200, mockResponse);

      const result = await client.flashbackDatabase({
        target_timestamp: '2024-12-22T10:00:00Z',
      });

      expect(result.duration_ms).toBeGreaterThan(0);
    });
  });

  // ==========================================================================
  // Flashback Statistics Tests
  // ==========================================================================

  describe('Flashback Statistics', () => {
    it('should get flashback statistics', async () => {
      const mockStats: FlashbackStatsResponse = {
        current_scn: 250000,
        oldest_scn: 50000,
        retention_days: 30,
        total_versions: 1500000,
        storage_bytes: 5368709120,
        queries_executed: 25000,
        restore_points: [
          {
            name: 'checkpoint_1',
            scn: 100000,
            timestamp: 1703721600,
            guaranteed: false,
          },
          {
            name: 'critical_point',
            scn: 200000,
            timestamp: 1703980800,
            guaranteed: true,
          },
        ],
      };

      mock.onGet('/api/v1/flashback/stats').reply(200, mockStats);

      const result = await client.getFlashbackStats();

      expect(result.current_scn).toBe(250000);
      expect(result.oldest_scn).toBe(50000);
      expect(result.retention_days).toBe(30);
      expect(result.total_versions).toBe(1500000);
      expect(result.restore_points).toHaveLength(2);
    });
  });

  // ==========================================================================
  // Transaction Flashback Tests
  // ==========================================================================

  describe('Transaction Flashback', () => {
    it('should reverse a transaction', async () => {
      const mockResponse: TransactionFlashbackResponse = {
        transaction_id: 'txn_bad_update',
        status: 'reversed',
        operations_reversed: 15,
        affected_tables: ['accounts', 'transactions'],
      };

      mock.onPost('/api/v1/flashback/transaction').reply(200, mockResponse);

      const result = await client.reverseTransaction('txn_bad_update', false);

      expect(result.transaction_id).toBe('txn_bad_update');
      expect(result.status).toBe('reversed');
      expect(result.operations_reversed).toBe(15);
      expect(result.affected_tables).toContain('accounts');
    });

    it('should reverse transaction with cascade', async () => {
      const mockResponse: TransactionFlashbackResponse = {
        transaction_id: 'txn_parent',
        status: 'reversed',
        operations_reversed: 50,
        affected_tables: ['orders', 'order_items', 'inventory'],
      };

      mock.onPost('/api/v1/flashback/transaction').reply(200, mockResponse);

      const result = await client.reverseTransaction('txn_parent', true);

      expect(result.operations_reversed).toBe(50);
      expect(result.affected_tables).toHaveLength(3);
    });

    it('should handle flashback transaction request', async () => {
      const mockResponse: TransactionFlashbackResponse = {
        transaction_id: 'txn_001',
        status: 'reversed',
        operations_reversed: 5,
        affected_tables: ['users'],
      };

      mock.onPost('/api/v1/flashback/transaction').reply(200, mockResponse);

      const result = await client.flashbackTransaction({
        transaction_id: 'txn_001',
        cascade: false,
      });

      expect(result.status).toBe('reversed');
    });
  });

  // ==========================================================================
  // Current SCN Tests
  // ==========================================================================

  describe('Current SCN', () => {
    it('should get current SCN', async () => {
      const currentSCN = 300000;

      mock.onGet('/api/v1/flashback/current-scn').reply(200, currentSCN);

      const result = await client.getCurrentSCN();

      expect(result).toBe(300000);
      expect(typeof result).toBe('number');
    });
  });

  // ==========================================================================
  // Utility Methods Tests
  // ==========================================================================

  describe('Utility Methods', () => {
    it('should wait for backup to complete', async () => {
      const backupId = 'backup_wait_test';

      // First call: in_progress
      mock
        .onGet(`/api/v1/backup/${backupId}`)
        .replyOnce(200, {
          backup_id: backupId,
          status: 'in_progress',
          backup_type: 'full',
          database_name: 'rustydb',
          start_time: 1704326400,
          location: '/backups/test',
          compression_enabled: true,
          encryption_enabled: true,
        })
        // Second call: completed
        .onGet(`/api/v1/backup/${backupId}`)
        .replyOnce(200, {
          backup_id: backupId,
          status: 'completed',
          backup_type: 'full',
          database_name: 'rustydb',
          start_time: 1704326400,
          completion_time: 1704330000,
          size_bytes: 104857600,
          location: '/backups/test',
          compression_enabled: true,
          encryption_enabled: true,
        });

      const result = await client.waitForBackup(backupId, {
        pollInterval: 100,
        timeout: 5000,
      });

      expect(result.status).toBe('completed');
    });

    it('should get backup statistics', async () => {
      const mockList: BackupList = {
        backups: [
          {
            backup_id: 'b1',
            backup_type: 'full',
            status: 'completed',
            start_time: 1,
            size_bytes: 1000,
            location: '/b1',
          },
          {
            backup_id: 'b2',
            backup_type: 'incremental',
            status: 'completed',
            start_time: 2,
            size_bytes: 500,
            location: '/b2',
          },
          {
            backup_id: 'b3',
            backup_type: 'full',
            status: 'failed',
            start_time: 3,
            location: '/b3',
          },
        ],
        total_count: 3,
      };

      mock.onGet('/api/v1/backup/list').reply(200, mockList);

      const stats = await client.getBackupStatistics();

      expect(stats.total).toBe(3);
      expect(stats.byType.full).toBe(2);
      expect(stats.byType.incremental).toBe(1);
      expect(stats.byStatus.completed).toBe(2);
      expect(stats.byStatus.failed).toBe(1);
      expect(stats.totalSize).toBe(1500);
    });

    it('should get oldest flashback time', async () => {
      const mockStats: FlashbackStatsResponse = {
        current_scn: 500000,
        oldest_scn: 100000,
        retention_days: 7,
        total_versions: 1000000,
        storage_bytes: 1073741824,
        queries_executed: 5000,
        restore_points: [],
      };

      mock.onGet('/api/v1/flashback/stats').reply(200, mockStats);

      const result = await client.getOldestFlashbackTime();

      expect(result.scn).toBe(100000);
      expect(result.retentionDays).toBe(7);
    });

    it('should check if flashback is possible', async () => {
      const mockStats: FlashbackStatsResponse = {
        current_scn: 500000,
        oldest_scn: 100000,
        retention_days: 30,
        total_versions: 1000000,
        storage_bytes: 1073741824,
        queries_executed: 5000,
        restore_points: [],
      };

      mock.onGet('/api/v1/flashback/stats').reply(200, mockStats);

      const recentTime = new Date(Date.now() - 24 * 60 * 60 * 1000); // 1 day ago
      const canFlashback = await client.canFlashbackTo(recentTime);

      expect(canFlashback).toBe(true);
    });
  });

  // ==========================================================================
  // Error Handling Tests
  // ==========================================================================

  describe('Error Handling', () => {
    it('should handle network errors', async () => {
      mock.onPost('/api/v1/backup/full').networkError();

      await expect(
        client.createFullBackup({
          backup_type: 'full',
        })
      ).rejects.toThrow();
    });

    it('should handle timeout errors', async () => {
      mock.onPost('/api/v1/backup/full').timeout();

      await expect(
        client.createFullBackup({
          backup_type: 'full',
        })
      ).rejects.toThrow();
    });

    it('should handle 500 server errors', async () => {
      mock.onPost('/api/v1/backup/full').reply(500, {
        code: 'INTERNAL_ERROR',
        message: 'Internal server error',
      });

      await expect(
        client.createFullBackup({
          backup_type: 'full',
        })
      ).rejects.toThrow();
    });

    it('should handle validation errors', async () => {
      mock.onPut('/api/v1/backup/schedule').reply(400, {
        code: 'INVALID_INPUT',
        message: 'retention_days must be greater than 0',
      });

      await expect(
        client.updateBackupSchedule({
          enabled: true,
          full_backup_cron: '0 2 * * 0',
          incremental_backup_cron: '0 2 * * 1-6',
          retention_days: 0, // Invalid
          compression: true,
          encryption: true,
          destination: '/backups',
        })
      ).rejects.toThrow();
    });
  });
});
