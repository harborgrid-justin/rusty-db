# Route Registration for Index and Memory Handlers

## Routes to Add to `/home/user/rusty-db/src/api/rest/server.rs`

Add these routes in the `build_router()` method, after the security labels routes and before `.with_state(self.state.clone())`:

```rust
// Index Management API
.route("/api/v1/indexes", get(index_handlers::list_indexes))
.route("/api/v1/indexes/{name}/stats", get(index_handlers::get_index_stats))
.route("/api/v1/indexes/{name}/rebuild", post(index_handlers::rebuild_index))
.route("/api/v1/indexes/{name}/analyze", post(index_handlers::analyze_index))
.route("/api/v1/indexes/recommendations", get(index_handlers::get_index_recommendations))

// Memory Management API
.route("/api/v1/memory/status", get(memory_handlers::get_memory_status))
.route("/api/v1/memory/allocator/stats", get(memory_handlers::get_allocator_stats))
.route("/api/v1/memory/gc", post(memory_handlers::trigger_gc))
.route("/api/v1/memory/pressure", get(memory_handlers::get_memory_pressure))
.route("/api/v1/memory/config", put(memory_handlers::update_memory_config))
```

## Location in server.rs

Insert after line containing:
```rust
.route("/api/v1/security/labels/classifications", get(labels_handlers::list_classifications))
```

And before line containing:
```rust
.with_state(self.state.clone());
```

## Handler Imports

The handler imports have already been added to server.rs:

```rust
// Index and Memory Handlers
use super::handlers::index_handlers;
use super::handlers::memory_handlers;
```

## API Endpoints Summary

### Index Management
- `GET /api/v1/indexes` - List all indexes
- `GET /api/v1/indexes/{name}/stats` - Get index statistics
- `POST /api/v1/indexes/{name}/rebuild` - Rebuild index
- `POST /api/v1/indexes/{name}/analyze` - Analyze index
- `GET /api/v1/indexes/recommendations` - Get index recommendations

### Memory Management
- `GET /api/v1/memory/status` - Get memory status
- `GET /api/v1/memory/allocator/stats` - Get allocator statistics
- `POST /api/v1/memory/gc` - Trigger garbage collection
- `GET /api/v1/memory/pressure` - Get memory pressure status
- `PUT /api/v1/memory/config` - Update memory configuration
