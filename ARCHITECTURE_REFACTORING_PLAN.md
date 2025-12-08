# RustyDB Architecture Refactoring Plan

## Executive Summary

This document outlines a comprehensive refactoring plan to align RustyDB with enterprise-grade Rust best practices. The codebase currently contains 100+ modules, with 20+ files exceeding 2000 lines.

## Critical Issues Identified

### 1. File Size & Module Cohesion (HIGH PRIORITY)

**Problem**: 20 files exceed 2000 LOC (target: 300-800 LOC)

**Top Offenders**:
- `src/execution/cte.rs` - **3166 LOC** - Contains 15+ distinct responsibilities
- `src/replication/mod.rs` - **3064 LOC** - Mixing replication logic, protocols, and management
- `src/api/rest_api.rs` - **3001 LOC** - All REST endpoints in one file
- `src/api/graphql_api.rs` - **2975 LOC** - All GraphQL resolvers in one file
- `src/analytics/mod.rs` - **2914 LOC** - Mixed analytics concerns
- `src/transaction/mod.rs` - **2877 LOC** - MVCC, isolation, deadlock detection combined
- `src/pool/session_manager.rs` - **2806 LOC** - Session, auth, privileges mixed
- `src/performance/mod.rs` - **2761 LOC** - Multiple performance subsystems
- `src/clustering/mod.rs` - **2736 LOC** - Cluster coordination, membership, health
- `src/memory/allocator.rs` - **2679 LOC** - Multiple allocation strategies
- `src/memory/buffer_pool.rs` - **2674 LOC** - Buffer management, eviction, I/O
- `src/network/advanced_protocol.rs` - **2666 LOC** - Protocol impl and message handling
- `src/storage/partitioning.rs` - **2520 LOC** - Multiple partitioning strategies
- `src/network/cluster_network.rs` - **2502 LOC** - Network layer and messaging
- `src/api/monitoring.rs` - **2428 LOC** - All monitoring endpoints
- `src/api/gateway.rs` - **2424 LOC** - Gateway logic and routing
- `src/pool/connection_pool.rs` - **2338 LOC** - Connection pooling and lifecycle
- `src/api/enterprise_integration.rs` - **2307 LOC** - Integration patterns

**Recommended Actions**:
1. Split each 2000+ LOC file into focused submodules
2. Create directory structures for complex domains
3. Use `mod.rs` for public API aggregation only

### 2. Primitive Obsession (HIGH PRIORITY)

**Problem**: Extensive use of raw `String` types for domain concepts

**Examples Found**:
```rust
// BEFORE (Weak typing)
pub sql_id: String,
pub task_name: String,
pub profile_name: String,
pub user_name: String,
pub table_name: String,
pub index_name: String,
pub wait_class: String,
pub wait_event: String,

// AFTER (Strong typing)
pub sql_id: SqlId,
pub task_name: TaskName,
pub profile_name: ProfileName,
pub user_name: UserName,
pub table_name: TableName,
pub index_name: IndexName,
pub wait_class: WaitClass,
pub wait_event: WaitEvent,
```

**Recommended Actions**:
1. Create `src/types/domain.rs` with newtype wrappers
2. Implement `Display`, `FromStr`, `Deref` for ergonomics
3. Add validation in constructors
4. Use `#[derive(serde::Serialize, serde::Deserialize)]` for wire format compatibility

### 3. Error Handling Maturity (MEDIUM PRIORITY)

**Current State**: Using custom `DbError` enum (good start)

**Improvements Needed**:
```rust
// Add context to errors
use anyhow::Context;

// Example
file_handle.read()
    .context("Failed to read transaction log")
    .context(format!("Transaction ID: {}", txn_id))?;

// Module-specific errors with thiserror
#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("Page {page_id} not found in buffer pool")]
    PageNotFound { page_id: PageId },
    
    #[error("Insufficient space: needed {needed}, available {available}")]
    InsufficientSpace { needed: usize, available: usize },
    
    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
}
```

### 4. Documentation Coverage (MEDIUM PRIORITY)

**Problem**: Inconsistent rustdoc coverage on public APIs

**Required Standard**:
```rust
/// Executes a recursive Common Table Expression (CTE).
///
/// # Arguments
///
/// * `cte_name` - The name of the CTE being evaluated
/// * `base_result` - Initial result set from the non-recursive term
/// * `recursive_fn` - Function that produces the next iteration
///
/// # Returns
///
/// Final materialized result after recursion completes or max iterations reached
///
/// # Errors
///
/// Returns `DbError::RecursionLimit` if max iterations exceeded
/// Returns `DbError::CycleDetected` if a cycle is found (when cycle detection enabled)
///
/// # Examples
///
/// ```rust
/// use rusty_db::execution::RecursiveCteEvaluator;
///
/// let evaluator = RecursiveCteEvaluator::new();
/// let result = evaluator.evaluate("ancestors", base_result, |prev| {
///     // Recursive step
///     query_next_level(prev)
/// })?;
/// ```
///
/// # Invariants
///
/// - `max_iterations` must be > 0
/// - `base_result` must have schema matching CTE definition
/// - Recursive function must eventually reach fixpoint or empty set
pub fn evaluate(&self, cte_name: &str, ...) -> Result<QueryResult> { ... }
```

### 5. Trait-Based Design (MEDIUM PRIORITY)

**Problem**: Concrete implementations without trait abstractions limit testability and extensibility

**Recommended Patterns**:
```rust
// Storage abstraction
pub trait StorageEngine: Send + Sync {
    fn read_page(&self, page_id: PageId) -> Result<Page>;
    fn write_page(&mut self, page: &Page) -> Result<()>;
    fn sync(&mut self) -> Result<()>;
}

// Allows swapping implementations
pub struct DiskStorage { ... }
pub struct InMemoryStorage { ... }
pub struct S3Storage { ... }

impl StorageEngine for DiskStorage { ... }
impl StorageEngine for InMemoryStorage { ... }
impl StorageEngine for S3Storage { ... }

// Usage with dependency injection
pub struct BufferPool<S: StorageEngine> {
    storage: S,
    ...
}
```

### 6. Test Coverage (HIGH PRIORITY)

**Problem**: Insufficient unit tests in complex modules

**Required Coverage**:
- Unit tests in `#[cfg(test)] mod tests {}` for each module
- Integration tests in `/tests` for end-to-end scenarios
- Property-based tests with `proptest` for invariants
- Concurrency tests for thread-safe modules

**Example Structure**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_cte_basic_evaluation() { ... }

    #[test]
    fn test_cte_recursion_limit() { ... }

    #[test]
    fn test_cte_cycle_detection() { ... }

    // Property-based test
    proptest! {
        #[test]
        fn test_cte_always_terminates(
            max_iter in 1usize..100,
            base_size in 0usize..1000
        ) {
            // Test that evaluation always terminates
        }
    }
}
```

## Refactoring Priority Matrix

### Phase 1 (Immediate - Week 1-2)
1. ✅ Split `execution/cte.rs` into submodules
2. ✅ Create domain types module with newtypes
3. ✅ Add rustdoc to 5 most-used public APIs
4. ✅ Create trait abstractions for storage layer

### Phase 2 (Short-term - Week 3-4)
5. Split `replication/mod.rs` into protocol/manager/state modules
6. Refactor `api/rest_api.rs` into endpoint-specific modules
7. Add comprehensive error context to transaction module
8. Add unit tests to execution module

### Phase 3 (Medium-term - Month 2)
9. Split remaining 2000+ LOC files
10. Replace all String domain types with newtypes
11. Add trait abstractions for network layer
12. Achieve 70%+ test coverage on core modules

### Phase 4 (Long-term - Month 3+)
13. Refactor all modules to <800 LOC
14. Complete rustdoc coverage
15. Add integration tests for all major workflows
16. Performance regression tests

## Module Decomposition Template

For each large file, follow this pattern:

```
src/execution/cte/
├── mod.rs              # Public API, re-exports
├── context.rs          # CteContext, registration
├── evaluator.rs        # RecursiveCteEvaluator
├── optimizer.rs        # Cost estimation, optimization
├── cycle_detector.rs   # Cycle detection logic
├── statistics.rs       # CTE statistics and reporting
├── dependency.rs       # Dependency graph analysis
└── tests.rs           # Module-level tests
```

## Code Quality Checklist

Before merging any refactored module:

- [ ] File size < 800 LOC (prefer < 500)
- [ ] Single Responsibility Principle maintained
- [ ] All public APIs have rustdoc with examples
- [ ] Domain types use newtypes, not primitives
- [ ] Error handling includes context
- [ ] Trait abstractions for extensibility points
- [ ] No unsafe blocks without documentation
- [ ] Unit test coverage > 70%
- [ ] Passes `cargo clippy -- -D warnings`
- [ ] Formatted with `cargo fmt`
- [ ] No performance regressions

## Next Steps

1. Review and approve this plan
2. Create GitHub issues for each refactoring task
3. Implement Phase 1 items
4. Measure and track metrics (LOC, test coverage, clippy warnings)
5. Iterate based on feedback

## Metrics Dashboard (To Be Tracked)

| Metric | Current | Target | Phase 1 Goal |
|--------|---------|--------|--------------|
| Files > 2000 LOC | 20 | 0 | 15 |
| Files > 1000 LOC | ~45 | 0 | 30 |
| Avg file size (LOC) | ~850 | 400-600 | 700 |
| Test coverage | ~25% | 80% | 40% |
| Clippy warnings | ~180 | 0 | 50 |
| String domain types | ~500+ | 0 | 250 |
| Modules w/ traits | ~15% | 70% | 30% |
| Rustdoc coverage | ~30% | 95% | 60% |

---

**Document Version**: 1.0  
**Created**: December 8, 2025  
**Next Review**: Weekly during Phase 1
