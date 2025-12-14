# WebSocket Test Data

This directory contains sample WebSocket event messages for testing replication and clustering features.

## Directory Structure

```
websocket/
├── replication/       # Replication events
│   ├── lag_alert.json
│   ├── replica_status_change.json
│   ├── conflict_detected.json
│   └── conflict_resolved.json
├── clustering/        # Cluster node events
│   ├── node_health_change.json
│   ├── failover_initiated.json
│   ├── failover_completed.json
│   └── leader_elected.json
├── rac/              # RAC-specific events
│   ├── cache_fusion_transfer.json
│   ├── lock_granted.json
│   ├── instance_recovery_started.json
│   └── parallel_query_completed.json
└── sharding/         # Sharding events
    ├── rebalance_progress.json
    └── shard_added.json
```

## Event Categories

### Replication Events

**Endpoint**: `ws://localhost:8080/api/v1/ws/cluster/replication`

Events streamed:
- `replication_lag_alert` - Lag exceeds threshold
- `replica_status_change` - Replica state changes
- `conflict_detected` - Replication conflict found
- `conflict_resolved` - Conflict resolution completed
- `wal_position_update` - WAL position updates
- `slot_created/dropped` - Replication slot changes

### Clustering Events

**Endpoint**: `ws://localhost:8080/api/v1/ws/cluster/nodes`

Events streamed:
- `node_joined/left` - Node membership changes
- `node_health_change` - Node health status changes
- `failover_initiated/completed` - Failover operations
- `leader_elected` - New leader elections
- `quorum_lost/restored` - Quorum status changes
- `migration_started/progress/completed` - Data migrations

### RAC Events

**Endpoint**: `ws://localhost:8080/api/v1/ws/cluster/rac`

Events streamed:
- `block_transfer` - Cache Fusion block transfers
- `lock_granted/released/converted` - Resource locks
- `recovery_started/progress/completed` - Instance recovery
- `parallel_query_started/completed` - Parallel queries
- `resource_remastered` - Resource master changes

### Sharding Events

**Endpoint**: `ws://localhost:8080/api/v1/ws/cluster/sharding`

Events streamed:
- `shard_added/removed` - Shard management
- `rebalance_started/progress/completed` - Shard rebalancing
- `shard_status_change` - Shard status updates

## Usage Examples

### Testing with wscat

```bash
# Install wscat if needed
npm install -g wscat

# Connect to replication events stream
wscat -c ws://localhost:8080/api/v1/ws/cluster/replication

# Send configuration to start receiving events
{"replica_ids": ["replica-001"], "include_lag_alerts": true}

# Connect to cluster events stream
wscat -c ws://localhost:8080/api/v1/ws/cluster/nodes

# Send configuration
{"node_ids": ["node-001"], "include_health_changes": true, "include_failover_events": true}
```

### Testing with JavaScript/TypeScript

```typescript
const ws = new WebSocket('ws://localhost:8080/api/v1/ws/cluster/replication');

ws.onopen = () => {
  // Send configuration
  ws.send(JSON.stringify({
    replica_ids: ['replica-001'],
    event_types: ['replication_lag_alert', 'replica_status_change'],
    min_severity: 'warning'
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received event:', data);

  // Handle different event types
  switch(data.event_type) {
    case 'replication_lag_alert':
      console.warn('Lag alert:', data.payload);
      break;
    case 'replica_status_change':
      console.log('Status changed:', data.payload);
      break;
  }
};
```

### Testing with cURL (for REST endpoints)

```bash
# Get WebSocket status
curl http://localhost:8080/api/v1/ws/status

# List active connections
curl http://localhost:8080/api/v1/ws/connections

# Broadcast message to connections
curl -X POST http://localhost:8080/api/v1/ws/broadcast \
  -H "Content-Type: application/json" \
  -d '{
    "event": "test_event",
    "message": {"text": "Test broadcast"},
    "target_connections": null
  }'
```

## Event Structure

All events follow the `ClusterEvent` envelope structure:

```json
{
  "category": "replication|clustering|rac|sharding",
  "event_type": "specific_event_type",
  "severity": "info|warning|error|critical",
  "source": "component:identifier",
  "timestamp": 1734134400,
  "event_id": "unique_event_id",
  "payload": {
    // Event-specific data
  }
}
```

## GraphQL Subscriptions

The same events are also available via GraphQL subscriptions:

```graphql
subscription {
  replicationLagUpdates(replicaId: "replica-001", threshold: 262144) {
    replicaId
    lagBytes
    lagSeconds
    severity
    timestamp
  }
}

subscription {
  nodeStatusChanges(nodeId: "node-001") {
    nodeId
    oldStatus
    newStatus
    role
    cpuUsage
    memoryUsage
    timestamp
  }
}

subscription {
  cacheFusionEvents(instanceId: "instance-1") {
    eventType
    blockId
    sourceInstance
    targetInstance
    blockMode
    transferSize
    durationMicros
    success
    timestamp
  }
}
```

## Testing Scenarios

### Scenario 1: Replication Lag Monitoring

1. Connect to `/api/v1/ws/cluster/replication`
2. Configure: `{"include_lag_alerts": true, "min_severity": "warning"}`
3. Watch for `replication_lag_alert` events
4. Verify threshold values and lag metrics

### Scenario 2: Cluster Health Monitoring

1. Connect to `/api/v1/ws/cluster/nodes`
2. Configure: `{"include_health_changes": true, "include_failover_events": true}`
3. Monitor node health changes
4. Watch for failover events during node failures

### Scenario 3: RAC Performance Monitoring

1. Connect to `/api/v1/ws/cluster/rac`
2. Configure: `{"include_cache_fusion": true, "include_lock_events": true}`
3. Monitor Cache Fusion transfers
4. Track lock grant/release patterns

### Scenario 4: Shard Rebalancing

1. Connect to `/api/v1/ws/cluster/sharding`
2. Configure: `{"tables": ["orders"], "include_rebalance_events": true}`
3. Monitor rebalance progress
4. Verify completion status

## Notes

- All timestamps are in Unix epoch seconds
- Event IDs are unique identifiers for each event
- Severity levels help prioritize event handling
- Test data files can be loaded and sent for integration testing
