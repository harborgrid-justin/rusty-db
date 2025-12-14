# WebSocket Query Execution Test Data

This directory contains comprehensive test data for WebSocket-based query execution monitoring in RustyDB.

## Overview

These JSON files provide real-world examples of WebSocket messages for:
- Query progress tracking
- Execution plan streaming
- Query cancellation
- Result set streaming
- CTE evaluation monitoring
- Parallel execution tracking
- Adaptive optimization events

## Files

### 1. query_progress_messages.json
Query progress updates showing execution progression.

**Message Type**: `query_progress`

**Example**:
```json
{
  "message_type": "query_progress",
  "query_id": "qry_abc123",
  "data": {
    "rows_scanned": 1000,
    "rows_returned": 500,
    "percentage_complete": 25.5,
    "current_operation": "Sequential Scan on users",
    "elapsed_ms": 1500,
    "estimated_remaining_ms": 4400
  }
}
```

### 2. execution_plan_messages.json
Execution plan node updates as query executes.

**Message Type**: `execution_plan_update`

**Plan Nodes**: SeqScan → HashJoin → Filter → Sort → Limit

### 3. cte_evaluation_messages.json
Common Table Expression evaluation events.

**Message Types**: `cte_evaluation`

**CTE Types**:
- Materialized CTEs
- Recursive CTEs (with iteration count)
- Inline CTEs

### 4. parallel_worker_messages.json
Parallel execution worker events.

**Message Type**: `parallel_worker`

**Event Types**:
- `started` - Worker initialization
- `progress` - Worker progress updates
- `completed` - Worker completion

### 5. adaptive_optimization_messages.json
Adaptive execution correction events.

**Message Type**: `adaptive_optimization`

**Correction Types**:
- Join order changes
- Join algorithm changes
- Index selection changes

### 6. query_cancellation_messages.json
Query cancellation requests and responses.

**Message Types**:
- Request: Query cancellation
- Response: `query_cancelled`

**Cancellation Reasons**:
- User-requested
- Timeout exceeded

### 7. result_streaming_messages.json
Large result set streaming in chunks.

**Message Types**:
- `result_chunk` - Individual result chunks
- `result_complete` - Streaming completion

**Features**:
- Progressive result delivery
- Chunk indexing
- Completion notification

## WebSocket Connection

### Connect to Query Execution Monitoring
```
ws://localhost:8080/api/v1/ws/query/execution
```

### Connect to Result Streaming
```
ws://localhost:8080/api/v1/ws/query/results
```

### Connect to CTE Monitoring
```
ws://localhost:8080/api/v1/ws/query/cte
```

### Connect to Parallel Execution Monitoring
```
ws://localhost:8080/api/v1/ws/query/parallel
```

### Connect to Adaptive Optimization Monitoring
```
ws://localhost:8080/api/v1/ws/query/adaptive
```

## Usage Examples

### JavaScript/Node.js
```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:8080/api/v1/ws/query/execution');

ws.on('open', () => {
  console.log('Connected to query execution monitoring');
});

ws.on('message', (data) => {
  const message = JSON.parse(data);

  switch(message.message_type) {
    case 'query_progress':
      console.log(`Progress: ${message.data.percentage_complete}%`);
      break;
    case 'execution_plan_update':
      console.log(`Plan node: ${message.data.plan_node}`);
      break;
    case 'query_cancelled':
      console.log(`Query cancelled: ${message.data.message}`);
      break;
  }
});
```

### Python
```python
import websocket
import json

def on_message(ws, message):
    msg = json.loads(message)
    msg_type = msg['message_type']

    if msg_type == 'query_progress':
        print(f"Progress: {msg['data']['percentage_complete']}%")
    elif msg_type == 'execution_plan_update':
        print(f"Plan node: {msg['data']['plan_node']}")

ws = websocket.WebSocketApp(
    "ws://localhost:8080/api/v1/ws/query/execution",
    on_message=on_message
)

ws.run_forever()
```

### Rust
```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8080/api/v1/ws/query/execution";
    let (ws_stream, _) = connect_async(url).await.unwrap();

    let (_, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            let data: serde_json::Value = serde_json::from_str(&text).unwrap();
            println!("Received: {}", data["message_type"]);
        }
    }
}
```

## Message Flow Examples

### Query Execution Flow
1. Connect to `/api/v1/ws/query/execution`
2. Receive `connected` message
3. Submit query via REST API
4. Receive `query_progress` messages (multiple)
5. Receive `execution_plan_update` messages (per plan node)
6. Optionally receive `adaptive_optimization` messages
7. Receive final `query_complete` message

### Query Cancellation Flow
1. Connect to `/api/v1/ws/query/execution`
2. Monitor query progress
3. Send `QueryCancellationRequest` message
4. Receive `query_cancelled` response

### Large Result Streaming Flow
1. Connect to `/api/v1/ws/query/results`
2. Submit query
3. Receive multiple `result_chunk` messages
4. Receive `result_complete` message

## Testing

Use these test files to:
- Validate WebSocket client implementations
- Test message parsing logic
- Simulate real-world query execution scenarios
- Performance test message handling
- Integration test query monitoring features

## Related Documentation

- Main Report: `/.scratchpad/agents/agent4_query_websocket_report.md`
- WebSocket Handlers: `/src/api/rest/handlers/query_websocket.rs`
- GraphQL Subscriptions: `/src/api/graphql/query_subscriptions.rs`
- REST Endpoints: `/src/api/rest/handlers/query_operations.rs`

## Notes

- All timestamps are Unix epoch seconds
- Message payloads use JSON format
- Binary WebSocket frames are not used
- Heartbeat/ping-pong handled automatically
- Connection timeout: 60 seconds (configurable)

---

**Last Updated**: 2025-12-14
**Agent**: PhD Engineer Agent 4
