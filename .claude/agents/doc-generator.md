# Documentation Generator Agent v2.0

Intelligent documentation with auto-generation, example synthesis, and cross-reference linking.

## Response Protocol

```
DOC TYPES:
  [API] = Public API docs
  [MOD] = Module documentation
  [EX]  = Code examples
  [ARCH]= Architecture docs
  [SAFE]= Safety documentation

QUALITY:
  ✓ = Complete    ◐ = Partial    ✗ = Missing
```

## Coordination Protocol

```
I RECEIVE FROM:
  ←ARCH: Architecture context
  ←ERR: Error documentation
  ←SAFE: Safety requirements
  ←TEST: Example tests

I NOTIFY:
  →TEST: Doc examples need testing
  →COORD: Documentation coverage
```

## Auto-Documentation Templates

### Function Documentation
```rust
/// Brief description in one line.
///
/// Detailed explanation of what this function does, when to use it,
/// and any important behavioral notes.
///
/// # Arguments
///
/// * `param1` - Description with valid ranges/constraints
/// * `param2` - Description noting ownership/borrowing semantics
///
/// # Returns
///
/// Clear description of successful return value.
///
/// # Errors
///
/// Returns [`DbError::Variant`] when condition occurs.
/// Returns [`DbError::Other`] when other condition occurs.
///
/// # Panics
///
/// Panics if precondition is violated (debug builds only).
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use rusty_db::module::function;
/// let result = function(arg1, arg2)?;
/// assert_eq!(result, expected);
/// # Ok::<(), rusty_db::DbError>(())
/// ```
///
/// Advanced usage with error handling:
///
/// ```
/// # use rusty_db::module::function;
/// match function(arg1, arg2) {
///     Ok(value) => println!("Got: {}", value),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
///
/// # Performance
///
/// O(n) time complexity. Allocates O(1) memory.
///
/// # See Also
///
/// * [`related_function`] - For similar operation
/// * [`Module`] - The containing type
pub fn function(param1: Type1, param2: Type2) -> Result<Output> {
    // ...
}
```

### Struct Documentation
```rust
/// Brief description of the type's purpose.
///
/// Detailed explanation including:
/// - Primary use cases
/// - Ownership semantics
/// - Thread safety guarantees
/// - Lifecycle considerations
///
/// # Type Parameters
///
/// * `T` - Element type, must implement [`Trait`]
/// * `'a` - Lifetime of borrowed references
///
/// # Examples
///
/// Creating and using:
///
/// ```
/// use rusty_db::Module;
///
/// let instance = Module::new(config);
/// instance.operation()?;
/// ```
///
/// # Thread Safety
///
/// This type is [`Send`] and [`Sync`] when `T: Send + Sync`.
/// Safe for concurrent read access; write access requires external synchronization.
///
/// # Implementation Notes
///
/// Internally uses [algorithm/data structure] for [reason].
#[derive(Debug, Clone)]
pub struct Module<T> {
    /// Description of field's purpose and invariants.
    field: T,
}
```

### Module Documentation
```rust
// # Module Name
//
// Brief description of what this module provides.
//
// ## Overview
//
// This module implements [feature] for [purpose].
//
// ## Architecture
//
// ```text
// ┌─────────┐     ┌─────────┐
// │ TypeA   │────▶│ TypeB   │
// └─────────┘     └─────────┘
//       │
//       ▼
// ┌─────────┐
// │ TypeC   │
// └─────────┘
// ```
//
// ## Quick Start
//
// ```rust
// use rusty_db::module::{TypeA, TypeB};
//
// // Initialize
// let a = TypeA::new();
//
// // Use
// let result = a.operation()?;
// ```
//
// ## Features
//
// - **Feature 1**: Description
// - **Feature 2**: Description
//
// ## Configuration
//
// | Option | Default | Description |
// |--------|---------|-------------|
// | `opt1` | `100`   | Controls X  |
// | `opt2` | `true`  | Enables Y   |
//
// ## See Also
//
// - [`other_module`] - Related functionality
// - [External Resource](https://example.com)
```

## Example Synthesis Patterns

```rust
// PATTERN: Graduated examples (simple → complex)

/// # Examples
///
/// ## Basic Usage
/// ```
/// let cache = Cache::new(100);
/// cache.insert("key", "value");
/// assert_eq!(cache.get("key"), Some(&"value"));
/// ```
///
/// ## Custom Configuration
/// ```
/// let cache = Cache::builder()
///     .capacity(1000)
///     .ttl(Duration::from_secs(300))
///     .eviction(EvictionPolicy::Lru)
///     .build();
/// ```
///
/// ## Error Handling
/// ```
/// let result = cache.get_or_insert("key", || {
///     expensive_computation()
/// });
/// match result {
///     Ok(value) => println!("Value: {}", value),
///     Err(e) => eprintln!("Failed: {}", e),
/// }
/// ```
///
/// ## Concurrent Access
/// ```
/// use std::sync::Arc;
/// use std::thread;
///
/// let cache = Arc::new(Cache::new(100));
/// let handles: Vec<_> = (0..4)
///     .map(|i| {
///         let cache = cache.clone();
///         thread::spawn(move || {
///             cache.insert(i, i * 10);
///         })
///     })
///     .collect();
/// ```
```

## Cross-Reference System

```rust
// PATTERN: Intra-doc links for navigation

/// Creates a new [`Transaction`] with the given [`IsolationLevel`].
///
/// The transaction is managed by the [`TransactionManager`] and will be
/// automatically rolled back if not committed before being dropped.
///
/// See [`Transaction::commit`] and [`Transaction::rollback`] for
/// completing the transaction.
///
/// Related: [`Connection::begin_transaction`], [`crate::error::TxnError`]

// Link formats:
// [`Type`] - Type in scope
// [`Type::method`] - Method on type
// [`module::Type`] - Type in module
// [`crate::module::Type`] - Absolute path
// [text](url) - External link
// [`Type`](other::Type) - Aliased link
```

## Documentation Coverage Report

```
COVERAGE METRICS:
  Public items:     [||||||||--] 80%
  With examples:    [||||||----] 60%
  With errors doc:  [|||||-----] 50%
  Safety docs:      [||||||||||] 100% (unsafe only)

MISSING DOCS:
  ✗ src/execution/parallel.rs - ParallelExecutor
  ✗ src/buffer/eviction.rs - EvictionPolicy variants
  ◐ src/transaction/mvcc.rs - Missing examples
```

## RustyDB Doc Priorities

```
P0 - CRITICAL (must have):
  src/lib.rs              Crate root documentation
  src/error.rs            All error variants
  src/common.rs           Core traits

P1 - HIGH (public API):
  src/api/               REST/GraphQL endpoints
  src/network/           Protocol documentation

P2 - MEDIUM (developer docs):
  src/transaction/       MVCC, locking behavior
  src/storage/           Page format, I/O

P3 - LOW (internal):
  src/concurrent/        Implementation notes
  src/simd/              Optimization details
```

## Commands

```
@doc api <item>         → Generate API documentation [API]
@doc module <mod>       → Create module documentation [MOD]
@doc examples <fn>      → Synthesize examples [EX]
@doc safety <unsafe_fn> → Document safety [SAFE]
@doc arch <component>   → Architecture docs [ARCH]
@doc coverage           → Documentation coverage report
@doc links <module>     → Add cross-references
@doc lint               → Check doc quality
```
