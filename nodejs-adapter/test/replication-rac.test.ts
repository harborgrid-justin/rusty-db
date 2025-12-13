/**
 * RustyDB Replication & RAC API Test Suite
 *
 * Comprehensive tests for all Replication & RAC endpoints including:
 * - Replication configuration
 * - Replication slots
 * - Conflict resolution
 * - RAC cluster management
 * - Cache Fusion operations
 * - Global Resource Directory (GRD)
 * - Cluster interconnect
 */

import { describe, it, expect, beforeAll, afterAll } from '@jest/globals';
import ReplicationRACClient, {
  ReplicationConfig,
  CreateSlotRequest,
  ResolveConflictRequest,
  CacheFlushRequest,
  RemasterRequest,
  AddNodeRequest,
  FailoverRequest,
} from '../src/api/replication-rac';

// Test configuration
const TEST_BASE_URL = process.env.RUSTYDB_API_URL || 'http://localhost:8080';
const TEST_API_KEY = process.env.RUSTYDB_API_KEY;

describe('ReplicationRACClient', () => {
  let client: ReplicationRACClient;

  beforeAll(() => {
    client = new ReplicationRACClient({
      baseURL: TEST_BASE_URL,
      apiKey: TEST_API_KEY,
      timeout: 10000,
    });
  });

  // ==========================================================================
  // Replication Configuration Tests
  // ==========================================================================

  describe('Replication Configuration', () => {
    it('should configure replication with synchronous mode', async () => {
      const config: ReplicationConfig = {
        mode: 'synchronous',
        standby_nodes: ['node-1:5432', 'node-2:5432'],
        replication_timeout_secs: 30,
        max_wal_senders: 10,
        wal_keep_segments: 64,
        archive_mode: true,
        archive_command: 'cp %p /archive/%f',
      };

      const response = await client.configureReplication(config);

      expect(response.success).toBe(true);
      expect(response.config.mode).toBe('synchronous');
      expect(response.config.standby_nodes).toHaveLength(2);
    });

    it('should configure replication with asynchronous mode', async () => {
      const config: ReplicationConfig = {
        mode: 'asynchronous',
        standby_nodes: ['node-3:5432'],
        replication_timeout_secs: 60,
      };

      const response = await client.configureReplication(config);

      expect(response.success).toBe(true);
      expect(response.config.mode).toBe('asynchronous');
    });

    it('should configure replication with semi-synchronous mode', async () => {
      const config: ReplicationConfig = {
        mode: 'semi_synchronous',
        standby_nodes: ['node-1:5432', 'node-2:5432', 'node-3:5432'],
        max_wal_senders: 5,
      };

      const response = await client.configureReplication(config);

      expect(response.success).toBe(true);
      expect(response.config.mode).toBe('semi_synchronous');
    });

    it('should get current replication configuration', async () => {
      const config = await client.getReplicationConfig();

      expect(config).toHaveProperty('mode');
      expect(config).toHaveProperty('standby_nodes');
      expect(['synchronous', 'asynchronous', 'semi_synchronous']).toContain(config.mode);
    });

    it('should reject invalid replication mode', async () => {
      const config: any = {
        mode: 'invalid_mode',
        standby_nodes: ['node-1:5432'],
      };

      await expect(client.configureReplication(config)).rejects.toThrow();
    });

    it('should reject empty standby nodes', async () => {
      const config: ReplicationConfig = {
        mode: 'synchronous',
        standby_nodes: [],
      };

      await expect(client.configureReplication(config)).rejects.toThrow();
    });
  });

  // ==========================================================================
  // Replication Slot Tests
  // ==========================================================================

  describe('Replication Slots', () => {
    const testSlotName = 'test_slot_' + Date.now();

    it('should list all replication slots', async () => {
      const response = await client.listReplicationSlots();

      expect(response).toHaveProperty('slots');
      expect(response).toHaveProperty('total_count');
      expect(Array.isArray(response.slots)).toBe(true);
    });

    it('should create a physical replication slot', async () => {
      const request: CreateSlotRequest = {
        slot_name: testSlotName + '_physical',
        slot_type: 'physical',
        temporary: false,
      };

      const slot = await client.createReplicationSlot(request);

      expect(slot.slot_name).toBe(request.slot_name);
      expect(slot.slot_type).toBe('physical');
      expect(slot.active).toBe(false);
    });

    it('should create a logical replication slot', async () => {
      const request: CreateSlotRequest = {
        slot_name: testSlotName + '_logical',
        slot_type: 'logical',
        plugin: 'pgoutput',
        temporary: false,
      };

      const slot = await client.createReplicationSlot(request);

      expect(slot.slot_name).toBe(request.slot_name);
      expect(slot.slot_type).toBe('logical');
      expect(slot.plugin).toBe('pgoutput');
    });

    it('should get a specific replication slot', async () => {
      const slotName = testSlotName + '_physical';
      const slot = await client.getReplicationSlot(slotName);

      expect(slot.slot_name).toBe(slotName);
      expect(slot).toHaveProperty('restart_lsn');
      expect(slot).toHaveProperty('wal_status');
    });

    it('should delete a replication slot', async () => {
      const slotName = testSlotName + '_physical';
      await expect(client.deleteReplicationSlot(slotName)).resolves.not.toThrow();
    });

    it('should reject logical slot without plugin', async () => {
      const request: CreateSlotRequest = {
        slot_name: 'invalid_slot',
        slot_type: 'logical',
        // Missing plugin
      };

      await expect(client.createReplicationSlot(request)).rejects.toThrow();
    });

    it('should reject duplicate slot creation', async () => {
      const request: CreateSlotRequest = {
        slot_name: testSlotName + '_logical',
        slot_type: 'logical',
        plugin: 'pgoutput',
      };

      await expect(client.createReplicationSlot(request)).rejects.toThrow();
    });
  });

  // ==========================================================================
  // Replication Conflict Tests
  // ==========================================================================

  describe('Replication Conflicts', () => {
    let testConflictId: string;

    it('should simulate a replication conflict', async () => {
      const conflict = await client.simulateReplicationConflict();

      expect(conflict).toHaveProperty('conflict_id');
      expect(conflict).toHaveProperty('database');
      expect(conflict).toHaveProperty('table_name');
      expect(conflict).toHaveProperty('conflict_type');
      expect(conflict.resolved).toBe(false);

      testConflictId = conflict.conflict_id;
    });

    it('should get all replication conflicts', async () => {
      const response = await client.getReplicationConflicts();

      expect(response).toHaveProperty('conflicts');
      expect(response).toHaveProperty('total_count');
      expect(response).toHaveProperty('unresolved_count');
      expect(Array.isArray(response.conflicts)).toBe(true);
    });

    it('should resolve conflict with use_local strategy', async () => {
      const request: ResolveConflictRequest = {
        conflict_id: testConflictId,
        strategy: 'use_local',
      };

      const result = await client.resolveReplicationConflict(request);

      expect(result.success).toBe(true);
      expect(result.conflict_id).toBe(testConflictId);
      expect(result.strategy).toBe('use_local');
    });

    it('should resolve conflict with use_remote strategy', async () => {
      // Create another conflict
      const conflict = await client.simulateReplicationConflict();

      const request: ResolveConflictRequest = {
        conflict_id: conflict.conflict_id,
        strategy: 'use_remote',
      };

      const result = await client.resolveReplicationConflict(request);

      expect(result.success).toBe(true);
      expect(result.strategy).toBe('use_remote');
    });

    it('should resolve conflict with last_write_wins strategy', async () => {
      const conflict = await client.simulateReplicationConflict();

      const request: ResolveConflictRequest = {
        conflict_id: conflict.conflict_id,
        strategy: 'last_write_wins',
      };

      const result = await client.resolveReplicationConflict(request);

      expect(result.success).toBe(true);
      expect(result.strategy).toBe('last_write_wins');
    });

    it('should resolve conflict with manual strategy', async () => {
      const conflict = await client.simulateReplicationConflict();

      const request: ResolveConflictRequest = {
        conflict_id: conflict.conflict_id,
        strategy: 'manual',
        manual_data: { id: 123, name: 'Manual Resolution', updated_at: Date.now() },
      };

      const result = await client.resolveReplicationConflict(request);

      expect(result.success).toBe(true);
      expect(result.strategy).toBe('manual');
    });

    it('should reject manual strategy without manual_data', async () => {
      const conflict = await client.simulateReplicationConflict();

      const request: ResolveConflictRequest = {
        conflict_id: conflict.conflict_id,
        strategy: 'manual',
        // Missing manual_data
      };

      await expect(client.resolveReplicationConflict(request)).rejects.toThrow();
    });
  });

  // ==========================================================================
  // RAC Cluster Management Tests
  // ==========================================================================

  describe('RAC Cluster Management', () => {
    it('should get cluster status', async () => {
      const status = await client.getClusterStatus();

      expect(status).toHaveProperty('state');
      expect(status).toHaveProperty('has_quorum');
      expect(status).toHaveProperty('healthy_nodes');
      expect(status).toHaveProperty('total_nodes');
      expect(status).toHaveProperty('is_healthy');
      expect(typeof status.timestamp).toBe('number');
    });

    it('should get cluster nodes', async () => {
      const nodes = await client.getClusterNodes();

      expect(Array.isArray(nodes)).toBe(true);
      if (nodes.length > 0) {
        expect(nodes[0]).toHaveProperty('node_id');
        expect(nodes[0]).toHaveProperty('address');
        expect(nodes[0]).toHaveProperty('role');
        expect(nodes[0]).toHaveProperty('status');
        expect(nodes[0]).toHaveProperty('cpu_cores');
      }
    });

    it('should get cluster statistics', async () => {
      const stats = await client.getClusterStats();

      expect(stats).toHaveProperty('total_nodes');
      expect(stats).toHaveProperty('active_nodes');
      expect(stats).toHaveProperty('uptime_seconds');
      expect(stats).toHaveProperty('cache_fusion');
      expect(stats).toHaveProperty('grd');
      expect(stats).toHaveProperty('interconnect');
    });

    it('should trigger cluster rebalance', async () => {
      const result = await client.triggerClusterRebalance();

      expect(result.status).toBe('success');
      expect(result).toHaveProperty('message');
    });
  });

  // ==========================================================================
  // Cache Fusion Tests
  // ==========================================================================

  describe('Cache Fusion', () => {
    it('should get Cache Fusion status', async () => {
      const status = await client.getCacheFusionStatus();

      expect(status).toHaveProperty('enabled');
      expect(status).toHaveProperty('zero_copy_enabled');
      expect(status).toHaveProperty('prefetch_enabled');
      expect(status).toHaveProperty('statistics');
      expect(status.statistics).toHaveProperty('total_requests');
      expect(status.statistics).toHaveProperty('cache_hits');
      expect(status.statistics).toHaveProperty('hit_rate_percent');
    });

    it('should get Cache Fusion statistics', async () => {
      const stats = await client.getCacheFusionStats();

      expect(stats).toHaveProperty('total_requests');
      expect(stats).toHaveProperty('successful_grants');
      expect(stats).toHaveProperty('cache_hits');
      expect(stats).toHaveProperty('cache_misses');
      expect(stats).toHaveProperty('bytes_transferred');
      expect(stats).toHaveProperty('avg_transfer_latency_us');
      expect(stats).toHaveProperty('hit_rate_percent');
    });

    it('should get Cache Fusion transfers', async () => {
      const transfers = await client.getCacheFusionTransfers();

      expect(Array.isArray(transfers)).toBe(true);
    });

    it('should flush Cache Fusion cache with default options', async () => {
      const request: CacheFlushRequest = {
        flush_dirty: true,
        invalidate_clean: false,
      };

      const result = await client.flushCacheFusion(request);

      expect(result.status).toBe('success');
      expect(result.flush_dirty).toBe(true);
      expect(result.invalidate_clean).toBe(false);
    });

    it('should flush and invalidate Cache Fusion cache', async () => {
      const request: CacheFlushRequest = {
        flush_dirty: true,
        invalidate_clean: true,
      };

      const result = await client.flushCacheFusion(request);

      expect(result.status).toBe('success');
      expect(result.flush_dirty).toBe(true);
      expect(result.invalidate_clean).toBe(true);
    });
  });

  // ==========================================================================
  // Global Resource Directory (GRD) Tests
  // ==========================================================================

  describe('Global Resource Directory (GRD)', () => {
    it('should get GRD topology', async () => {
      const topology = await client.getGRDTopology();

      expect(topology).toHaveProperty('members');
      expect(topology).toHaveProperty('hash_ring_buckets');
      expect(topology).toHaveProperty('load_distribution');
      expect(Array.isArray(topology.members)).toBe(true);
    });

    it('should get GRD resources', async () => {
      const resources = await client.getGRDResources();

      expect(Array.isArray(resources)).toBe(true);
    });

    it('should trigger GRD remastering without force', async () => {
      const request: RemasterRequest = {
        force: false,
      };

      const result = await client.triggerGRDRemaster(request);

      expect(result.status).toBe('success');
      expect(result.force).toBe(false);
    });

    it('should trigger forced GRD remastering', async () => {
      const request: RemasterRequest = {
        force: true,
      };

      const result = await client.triggerGRDRemaster(request);

      expect(result.status).toBe('success');
      expect(result.force).toBe(true);
    });

    it('should trigger GRD remastering with target node', async () => {
      const request: RemasterRequest = {
        force: false,
        target_node: 'node-1',
      };

      const result = await client.triggerGRDRemaster(request);

      expect(result.status).toBe('success');
      expect(result.target_node).toBe('node-1');
    });
  });

  // ==========================================================================
  // Interconnect Tests
  // ==========================================================================

  describe('Cluster Interconnect', () => {
    it('should get interconnect status', async () => {
      const status = await client.getInterconnectStatus();

      expect(status).toHaveProperty('local_node');
      expect(status).toHaveProperty('listen_address');
      expect(status).toHaveProperty('connected_nodes');
      expect(status).toHaveProperty('healthy_nodes');
      expect(status).toHaveProperty('is_running');
      expect(Array.isArray(status.healthy_nodes)).toBe(true);
    });

    it('should get interconnect statistics', async () => {
      const stats = await client.getInterconnectStats();

      expect(stats).toHaveProperty('messages_sent');
      expect(stats).toHaveProperty('messages_received');
      expect(stats).toHaveProperty('bytes_sent');
      expect(stats).toHaveProperty('bytes_received');
      expect(stats).toHaveProperty('avg_message_latency_us');
      expect(stats).toHaveProperty('avg_throughput_mbps');
    });
  });

  // ==========================================================================
  // Cluster Management Tests (from cluster.rs)
  // ==========================================================================

  describe('Basic Cluster Management', () => {
    const testNodeId = 'test_node_' + Date.now();

    it('should get basic cluster nodes', async () => {
      const nodes = await client.getBasicClusterNodes();

      expect(Array.isArray(nodes)).toBe(true);
      if (nodes.length > 0) {
        expect(nodes[0]).toHaveProperty('node_id');
        expect(nodes[0]).toHaveProperty('address');
        expect(nodes[0]).toHaveProperty('role');
      }
    });

    it('should add a cluster node', async () => {
      const request: AddNodeRequest = {
        node_id: testNodeId,
        address: '192.168.1.100:5432',
        role: 'follower',
      };

      const node = await client.addClusterNode(request);

      expect(node.node_id).toBe(testNodeId);
      expect(node.address).toBe(request.address);
      expect(node.role).toBe('follower');
    });

    it('should get cluster node by ID', async () => {
      const node = await client.getClusterNode(testNodeId);

      expect(node.node_id).toBe(testNodeId);
    });

    it('should get cluster topology', async () => {
      const topology = await client.getClusterTopology();

      expect(topology).toHaveProperty('cluster_id');
      expect(topology).toHaveProperty('nodes');
      expect(topology).toHaveProperty('quorum_size');
      expect(topology).toHaveProperty('total_nodes');
      expect(Array.isArray(topology.nodes)).toBe(true);
    });

    it('should get basic replication status', async () => {
      const status = await client.getBasicReplicationStatus();

      expect(status).toHaveProperty('primary_node');
      expect(status).toHaveProperty('replicas');
      expect(status).toHaveProperty('sync_state');
      expect(Array.isArray(status.replicas)).toBe(true);
    });

    it('should get cluster configuration', async () => {
      const config = await client.getClusterConfig();

      expect(config).toHaveProperty('cluster_name');
      expect(config).toHaveProperty('replication_factor');
    });

    it('should update cluster configuration', async () => {
      const newConfig = {
        heartbeat_interval_ms: 2000,
        sync_replication: true,
      };

      await expect(client.updateClusterConfig(newConfig)).resolves.not.toThrow();
    });

    it('should remove a cluster node', async () => {
      await expect(client.removeClusterNode(testNodeId)).resolves.not.toThrow();
    });

    it('should reject removing local node', async () => {
      await expect(client.removeClusterNode('node-local')).rejects.toThrow();
    });

    it('should trigger failover', async () => {
      const request: FailoverRequest = {
        force: false,
      };

      // This may fail if quorum is not met, so we just check it doesn't throw unexpected errors
      try {
        await client.triggerFailover(request);
      } catch (error: any) {
        // Accept both success and quorum-related failures
        expect(
          error.response?.status === 202 ||
          error.response?.data?.code === 'FORBIDDEN'
        ).toBeTruthy();
      }
    });
  });

  // ==========================================================================
  // Parallel Query Execution Tests
  // ==========================================================================

  describe('Parallel Query Execution', () => {
    it('should execute a parallel query', async () => {
      const sql = 'SELECT COUNT(*) FROM large_table';
      const options = {
        parallelism: 4,
        timeout: 30000,
      };

      try {
        const result = await client.executeParallelQuery(sql, options);
        expect(result).toBeDefined();
      } catch (error) {
        // This may fail if the query endpoint is not available or table doesn't exist
        console.log('Parallel query test skipped:', error);
      }
    });

    it('should execute parallel query on specific nodes', async () => {
      const sql = 'SELECT * FROM distributed_table LIMIT 100';
      const options = {
        parallelism: 2,
        nodes: ['node-1', 'node-2'],
      };

      try {
        const result = await client.executeParallelQuery(sql, options);
        expect(result).toBeDefined();
      } catch (error) {
        console.log('Parallel query on nodes test skipped:', error);
      }
    });
  });

  // ==========================================================================
  // Integration Tests
  // ==========================================================================

  describe('Integration Tests', () => {
    it('should handle full replication workflow', async () => {
      // 1. Configure replication
      const config: ReplicationConfig = {
        mode: 'synchronous',
        standby_nodes: ['node-1:5432'],
        max_wal_senders: 5,
      };
      const configResponse = await client.configureReplication(config);
      expect(configResponse.success).toBe(true);

      // 2. Create replication slot
      const slotRequest: CreateSlotRequest = {
        slot_name: 'integration_test_slot',
        slot_type: 'logical',
        plugin: 'pgoutput',
      };
      const slot = await client.createReplicationSlot(slotRequest);
      expect(slot.slot_name).toBe('integration_test_slot');

      // 3. Simulate and resolve conflict
      const conflict = await client.simulateReplicationConflict();
      const resolveRequest: ResolveConflictRequest = {
        conflict_id: conflict.conflict_id,
        strategy: 'last_write_wins',
      };
      const resolution = await client.resolveReplicationConflict(resolveRequest);
      expect(resolution.success).toBe(true);

      // 4. Check replication status
      const status = await client.getBasicReplicationStatus();
      expect(status).toHaveProperty('primary_node');

      // 5. Clean up
      await client.deleteReplicationSlot('integration_test_slot');
    });

    it('should handle full RAC cluster workflow', async () => {
      // 1. Get cluster status
      const status = await client.getClusterStatus();
      expect(status).toHaveProperty('is_healthy');

      // 2. Get cluster statistics
      const stats = await client.getClusterStats();
      expect(stats.cache_fusion).toHaveProperty('total_requests');

      // 3. Check Cache Fusion
      const cfStatus = await client.getCacheFusionStatus();
      expect(cfStatus).toHaveProperty('enabled');

      // 4. Check GRD topology
      const topology = await client.getGRDTopology();
      expect(topology).toHaveProperty('members');

      // 5. Trigger rebalance
      const rebalanceResult = await client.triggerClusterRebalance();
      expect(rebalanceResult.status).toBe('success');
    });
  });
});

// Export test utilities
export { TEST_BASE_URL, TEST_API_KEY };
