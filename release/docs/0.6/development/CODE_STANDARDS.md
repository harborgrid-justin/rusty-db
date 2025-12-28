# RustyDB v0.6.0 - Code Standards

**Version**: 0.6.0
**Release**: $856M Enterprise Server
**Last Updated**: 2025-12-28

---

## Table of Contents

1. [Overview](#overview)
2. [Rust Style Guidelines](#rust-style-guidelines)
3. [Naming Conventions](#naming-conventions)
4. [Code Organization](#code-organization)
5. [Error Handling](#error-handling)
6. [Documentation Standards](#documentation-standards)
7. [Testing Standards](#testing-standards)
8. [Security Guidelines](#security-guidelines)
9. [Performance Patterns](#performance-patterns)
10. [Module Development Patterns](#module-development-patterns)

---

## Overview

This document defines the coding standards and best practices for RustyDB development. All code must adhere to these standards to ensure consistency, maintainability, and quality.

### Core Principles

1. **Type Safety**: Leverage Rust's type system for compile-time correctness
2. **Explicit Over Implicit**: Make intentions clear in code
3. **Error Handling**: Use `Result<T>` for all fallible operations
4. **Documentation**: Document all public APIs with examples
5. **Testing**: Write tests alongside code
6. **Performance**: Profile before optimizing
7. **Security**: Validate all inputs, use safe abstractions

---

## Rust Style Guidelines

### Follow Rust API Guidelines

All code must follow the official [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

### Code Formatting

**Use `rustfmt`** for all code formatting:

```bash
# Format entire project
cargo fmt

# Check formatting without modifying
cargo fmt -- --check
```

**Configuration** (`.rustfmt.toml`):
```toml
max_width = 100
hard_tabs = false
tab_spaces = 4
edition = "2021"
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
```

### Linting with Clippy

**Run Clippy** on all code before committing:

```bash
# Run clippy
cargo clippy

# Fail on warnings (CI/CD)
cargo clippy -- -D warnings

# Auto-fix issues
cargo clippy --fix
```

**Clippy Configuration** (`Cargo.toml`):
```toml
[lints.clippy]
pedantic = "warn"
nursery = "warn"
cargo = "warn"
```

---

## Naming Conventions

### General Rules

| Item | Convention | Example |
|------|------------|---------|
| **Crates** | snake_case | `rusty_db` |
| **Modules** | snake_case | `buffer_pool` |
| **Types** | PascalCase | `BufferPool` |
| **Traits** | PascalCase | `Transactional` |
| **Enums** | PascalCase | `LockMode` |
| **Enum Variants** | PascalCase | `LockMode::Exclusive` |
| **Structs** | PascalCase | `PageGuard` |
| **Functions** | snake_case | `pin_page` |
| **Methods** | snake_case | `get_buffer_pool` |
| **Local Variables** | snake_case | `page_id` |
| **Constants** | SCREAMING_SNAKE_CASE | `PAGE_SIZE` |
| **Static Variables** | SCREAMING_SNAKE_CASE | `MAX_CONNECTIONS` |
| **Type Parameters** | PascalCase | `T`, `K`, `V` |
| **Lifetimes** | lowercase | `'a`, `'static` |

### Specific Guidelines

**Functions**:
```rust
// Good
fn calculate_checksum(data: &[u8]) -> u32 { }
fn is_valid_page(page_id: PageId) -> bool { }

// Bad
fn CalcChecksum(data: &[u8]) -> u32 { }
fn ValidPage(page_id: PageId) -> bool { }
```

**Types**:
```rust
// Good
struct BufferPool { }
enum LockMode { }
trait Transactional { }

// Bad
struct buffer_pool { }
enum lockMode { }
trait transactional { }
```

**Constants**:
```rust
// Good
const PAGE_SIZE: usize = 4096;
const MAX_CONNECTIONS: usize = 100;

// Bad
const page_size: usize = 4096;
const MaxConnections: usize = 100;
```

**Boolean Predicates**:
```rust
// Good - use is_, has_, can_ prefix
fn is_empty(&self) -> bool { }
fn has_next(&self) -> bool { }
fn can_commit(&self) -> bool { }

// Bad
fn empty(&self) -> bool { }
fn next(&self) -> bool { }
fn commit_allowed(&self) -> bool { }
```

**Conversions**:
```rust
// Good - follow standard naming
fn as_bytes(&self) -> &[u8] { }      // Cheap reference conversion
fn to_string(&self) -> String { }     // Expensive owned conversion
fn into_inner(self) -> T { }          // Consuming conversion
fn from_bytes(bytes: &[u8]) -> Self { } // Constructor

// Bad
fn get_bytes(&self) -> &[u8] { }
fn make_string(&self) -> String { }
```

---

## Code Organization

### Line Length

**Maximum**: 100 characters per line

### Indentation

**Use 4 spaces** (enforced by rustfmt)

### Import Organization

```rust
// Standard library imports
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// External crate imports
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};

// Internal crate imports
use crate::error::{DbError, Result};
use crate::common::{PageId, TransactionId};
use crate::storage::Page;

// Module imports
use super::buffer_pool::BufferPool;
use self::cache::Cache;
```

### Module Structure

**Recommended file size**: <500 lines

**Module organization**:
```
module/
├── mod.rs          # Public API, re-exports
├── types.rs        # Type definitions
├── core.rs         # Core implementation
├── operations.rs   # Operations
└── utils.rs        # Utilities
```

**Example `mod.rs`**:
```rust
//! Module for doing X.
//!
//! This module provides functionality for...

mod core;
mod types;
mod operations;
mod utils;

// Re-export public items
pub use types::{MyType, MyEnum};
pub use core::MyStruct;
pub use operations::{my_function, another_function};

// Internal visibility
pub(crate) use utils::internal_helper;

#[cfg(test)]
mod tests;
```

---

## Error Handling

### Use the Unified Error Type

All functions return `Result<T>` using `DbError`:

```rust
use crate::error::{DbError, Result};

pub fn my_function() -> Result<()> {
    // Operations that may fail
    some_operation()?;
    Ok(())
}
```

### Error Propagation

**Use the `?` operator**:

```rust
// Good
pub fn read_page(page_id: PageId) -> Result<Page> {
    let data = self.disk.read(page_id)?;
    let page = Page::deserialize(&data)?;
    Ok(page)
}

// Bad
pub fn read_page(page_id: PageId) -> Result<Page> {
    match self.disk.read(page_id) {
        Ok(data) => match Page::deserialize(&data) {
            Ok(page) => Ok(page),
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    }
}
```

### Error Context

**Provide context for errors**:

```rust
use anyhow::Context;

let page = self.disk.read(page_id)
    .context(format!("Failed to read page {}", page_id))?;
```

### Custom Error Variants

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Page {0} not found")]
    PageNotFound(PageId),

    #[error("Transaction {0} already committed")]
    TransactionCommitted(TransactionId),

    #[error("Lock timeout after {timeout}ms")]
    LockTimeout { timeout: u64 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Error Handling Patterns

```rust
// Match on specific errors
match operation() {
    Ok(result) => { /* handle success */ },
    Err(DbError::PageNotFound(_)) => { /* handle missing page */ },
    Err(DbError::LockTimeout { .. }) => { /* handle timeout */ },
    Err(e) => return Err(e), // Propagate other errors
}

// Unwrap only in tests or when guaranteed safe
let value = operation().unwrap(); // Only in tests!

// Use expect() with explanation
let value = operation().expect("Buffer pool should be initialized");

// Provide defaults
let value = operation().unwrap_or_default();
let value = operation().unwrap_or(42);
```

---

## Documentation Standards

### Module-Level Documentation

```rust
//! Buffer pool management module.
//!
//! This module provides a high-performance buffer pool for caching
//! database pages in memory. It supports multiple eviction policies
//! including CLOCK, LRU, 2Q, LRU-K, LIRS, and ARC.
//!
//! # Examples
//!
//! ```
//! use rusty_db::buffer::BufferPool;
//!
//! let pool = BufferPool::new(1000);
//! let page = pool.pin_page(1)?;
//! // Use page...
//! // Page is automatically unpinned when it goes out of scope
//! ```
//!
//! # Architecture
//!
//! The buffer pool consists of:
//! - Frame table: Physical memory for pages
//! - Page table: Mapping from page IDs to frames
//! - Eviction policy: Determines which page to evict
```

### Function Documentation

```rust
/// Pins a page in the buffer pool.
///
/// This function loads the page from disk if not already in memory,
/// pins it to prevent eviction, and returns a guard that automatically
/// unpins the page when dropped.
///
/// # Arguments
///
/// * `page_id` - The ID of the page to pin
///
/// # Returns
///
/// A pinned page guard that provides access to the page data.
///
/// # Errors
///
/// Returns `DbError::PageNotFound` if the page doesn't exist.
/// Returns `DbError::Io` if disk I/O fails.
///
/// # Examples
///
/// ```
/// let page = buffer_pool.pin_page(1)?;
/// let data = page.read();
/// // Page is automatically unpinned when `page` goes out of scope
/// ```
///
/// # Panics
///
/// Panics if the buffer pool is full and no pages can be evicted.
pub fn pin_page(&self, page_id: PageId) -> Result<PageGuard> {
    // Implementation
}
```

### Type Documentation

```rust
/// A buffer pool for caching database pages in memory.
///
/// The buffer pool manages a fixed-size pool of page frames and
/// provides efficient page access through pinning and unpinning.
///
/// # Thread Safety
///
/// This type is `Send + Sync` and can be safely shared across threads.
///
/// # Examples
///
/// ```
/// let pool = BufferPool::new(1000);
/// let page = pool.pin_page(1)?;
/// ```
pub struct BufferPool {
    // Fields...
}
```

### Inline Comments

```rust
// Good - explain WHY, not WHAT
// Use arc-swap for lock-free page table updates
let page_table = ArcSwap::new(Arc::new(HashMap::new()));

// Bad - obvious from code
// Increment counter
counter += 1;

// Good - explain complex logic
// We need to check if the page is dirty before eviction because
// dirty pages must be flushed to disk to maintain durability.
if page.is_dirty() {
    self.flush_page(page_id)?;
}
```

---

## Testing Standards

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Test fixtures
    fn setup_buffer_pool() -> BufferPool {
        BufferPool::new(100)
    }

    // Happy path tests
    #[test]
    fn test_pin_page_success() {
        let pool = setup_buffer_pool();
        let page = pool.pin_page(1).unwrap();
        assert_eq!(page.id(), 1);
    }

    // Error cases
    #[test]
    fn test_pin_page_not_found() {
        let pool = setup_buffer_pool();
        let result = pool.pin_page(9999);
        assert!(matches!(result, Err(DbError::PageNotFound(_))));
    }

    // Edge cases
    #[test]
    fn test_pin_page_buffer_full() {
        let pool = setup_buffer_pool();
        // Fill buffer pool...
        // Verify eviction works correctly
    }

    // Concurrent tests
    #[tokio::test]
    async fn test_concurrent_access() {
        let pool = Arc::new(setup_buffer_pool());
        // Spawn multiple tasks...
    }
}
```

### Test Naming

```rust
// Good - descriptive test names
#[test]
fn test_pin_page_returns_correct_data() { }

#[test]
fn test_eviction_chooses_least_recently_used() { }

#[test]
fn test_concurrent_pin_same_page_succeeds() { }

// Bad - vague test names
#[test]
fn test1() { }

#[test]
fn test_page() { }
```

### Assertion Patterns

```rust
// Use assert_eq! for equality
assert_eq!(actual, expected);

// Use assert! for boolean conditions
assert!(value > 0);
assert!(!list.is_empty());

// Use matches! for enum variants
assert!(matches!(result, Ok(Page { .. })));
assert!(matches!(error, Err(DbError::PageNotFound(_))));

// Use assert_ne! for inequality
assert_ne!(page1.id(), page2.id());
```

---

## Security Guidelines

### Input Validation

```rust
/// Always validate user input
pub fn create_table(name: &str) -> Result<()> {
    // Validate length
    if name.is_empty() || name.len() > MAX_TABLE_NAME_LEN {
        return Err(DbError::InvalidTableName);
    }

    // Validate format
    if !is_valid_identifier(name) {
        return Err(DbError::InvalidTableName);
    }

    // Prevent SQL injection
    if contains_sql_keywords(name) {
        return Err(DbError::InvalidTableName);
    }

    // Proceed with table creation
    Ok(())
}
```

### SQL Injection Prevention

```rust
// Good - use parameterized queries
let stmt = "SELECT * FROM users WHERE id = ?";
let result = executor.execute(stmt, &[&user_id])?;

// Bad - never concatenate user input
// let stmt = format!("SELECT * FROM users WHERE id = {}", user_id);
```

### Sensitive Data

```rust
#[derive(Debug)]
pub struct User {
    pub id: UserId,
    pub name: String,
    #[sensitive]  // Don't log this
    pub password_hash: String,
}

// Zeroize on drop
impl Drop for User {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.password_hash.zeroize();
    }
}
```

### Secure Coding Practices

1. **Never trust user input** - Validate all external data
2. **Use prepared statements** - Prevent SQL injection
3. **Encrypt sensitive data** - At rest and in transit
4. **Validate all external data** - Check bounds, types, formats
5. **Minimize privileges** - Principle of least privilege
6. **Audit security-critical operations** - Log access attempts
7. **Keep dependencies updated** - Run `cargo audit` regularly

---

## Performance Patterns

### Use SIMD Where Applicable

```rust
#[cfg(feature = "simd")]
use packed_simd::*;

#[cfg(feature = "simd")]
fn filter_simd(data: &[i32], threshold: i32) -> Vec<i32> {
    // SIMD implementation
}

#[cfg(not(feature = "simd"))]
fn filter_simd(data: &[i32], threshold: i32) -> Vec<i32> {
    // Fallback implementation
    data.iter().filter(|&&x| x > threshold).copied().collect()
}
```

### Minimize Allocations

```rust
// Good - pre-allocate with known capacity
let mut results = Vec::with_capacity(expected_size);

// Good - reuse allocations
let mut buffer = Vec::new();
for _ in 0..iterations {
    buffer.clear();
    // Use buffer...
}

// Bad - unnecessary allocation
for _ in 0..iterations {
    let buffer = Vec::new();  // Allocates each iteration!
}
```

### Avoid Unnecessary Clones

```rust
// Good - use references
fn process_data(data: &[u8]) -> Result<()> {
    // Work with reference
}

// Good - use Cow for sometimes-owned data
use std::borrow::Cow;

fn process_string(s: Cow<str>) -> String {
    // Can use borrowed or owned string
}

// Bad - unnecessary clone
fn process_data(data: Vec<u8>) -> Result<()> {
    // Caller must clone to keep data
}
```

### Lock-Free Data Structures

```rust
// Use lock-free structures from concurrent/ module for hot paths
use crate::concurrent::LockFreeQueue;

// Prefer arc-swap for read-heavy, write-light scenarios
use arc_swap::ArcSwap;

let shared_state = Arc::new(ArcSwap::new(Arc::new(initial_state)));
```

### Profile Before Optimizing

```rust
// Always measure before optimizing
// Use flamegraph for CPU profiling
// Use heaptrack for memory profiling

// Example: Add criterion benchmark
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_operation(c: &mut Criterion) {
    c.bench_function("operation", |b| {
        b.iter(|| {
            expensive_operation(black_box(42))
        })
    });
}
```

---

## Module Development Patterns

### Component Lifecycle

```rust
use crate::common::{Component, HealthStatus};
use crate::error::Result;

impl Component for MyComponent {
    fn initialize(&mut self) -> Result<()> {
        // Initialize resources
        self.connection_pool.start()?;
        self.worker_threads.spawn()?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Clean up resources
        self.worker_threads.stop()?;
        self.connection_pool.shutdown()?;
        Ok(())
    }

    fn health_check(&self) -> HealthStatus {
        // Return component health
        if self.is_healthy() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy(format!("Component degraded: {}", self.error))
        }
    }
}
```

### Async/Await Patterns

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn async_operation() -> Result<()> {
    // Use tokio runtime for async operations
    let mut file = tokio::fs::File::open("data.db").await?;
    let mut buffer = vec![0; 4096];
    file.read_exact(&mut buffer).await?;
    Ok(())
}

// Use tokio::spawn for concurrent tasks
let handle = tokio::spawn(async move {
    process_data().await
});

let result = handle.await??;
```

### Thread Safety

```rust
// Use Arc<Mutex<T>> for shared mutable state
use std::sync::{Arc, Mutex};

let shared_data = Arc::new(Mutex::new(Vec::new()));

// Use Arc<RwLock<T>> for read-heavy scenarios
use std::sync::RwLock;

let shared_data = Arc::new(RwLock::new(HashMap::new()));

// Prefer lock-free data structures where possible
use arc_swap::ArcSwap;

let shared_state = Arc::new(ArcSwap::new(Arc::new(State::new())));
```

### Module Dependencies

**Rules**:
1. All modules depend on `error` and `common`
2. Lower layers don't depend on higher layers
3. Avoid circular dependencies
4. Use traits for dependency inversion

**Dependency Graph**:
```
execution → transaction → storage → buffer → io
    ↓           ↓           ↓         ↓       ↓
  error ← ← ← ← common ← ← ← ← ← ← ← ← ← ← ←
```

---

## Common Issues and Solutions

### Import Issues

```rust
// Use crate:: for absolute imports within the project
use crate::storage::Page;

// Use super:: for parent module imports
use super::buffer_pool::BufferPool;

// Use self:: for current module imports
use self::cache::Cache;
```

### Type Mismatches

```rust
// Use .into() for type conversions
let page_id: PageId = 42_u64.into();

// Use .try_into() for fallible conversions
let page_id: PageId = value.try_into()?;

// Ensure generic type parameters are correctly specified
let result: Result<Page, DbError> = operation();
```

### Trait Bounds

```rust
// Use where clauses for complex bounds
fn generic_function<T>(value: T) -> Result<()>
where
    T: Send + Sync + Clone,
    T: Serialize + DeserializeOwned,
{
    // Implementation
}

// Import traits that provide methods
use std::io::Write;  // Needed for write_all()
```

### Lifetime Issues

```rust
// Use explicit lifetime annotations where needed
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

// Prefer owned types when lifetime is unclear
fn process_data(data: Vec<u8>) -> Result<String> {
    // Work with owned data
}

// Use 'static for data that lives for program duration
const GLOBAL_CONFIG: &'static str = "config";
```

---

## Code Review Checklist

Before submitting code for review, verify:

- [ ] Code follows Rust API guidelines
- [ ] All functions have documentation comments
- [ ] Tests are written and passing
- [ ] `cargo fmt` has been run
- [ ] `cargo clippy` passes without warnings
- [ ] Error handling uses `Result<T>`
- [ ] Security: All inputs validated
- [ ] Performance: No obvious inefficiencies
- [ ] No hardcoded values (use constants)
- [ ] Module dependencies are correct
- [ ] Thread safety considered for concurrent code
- [ ] Documentation updated
- [ ] Examples provided for public APIs

---

**For more information, see [DEVELOPMENT_OVERVIEW.md](./DEVELOPMENT_OVERVIEW.md) and [CONTRIBUTING.md](./CONTRIBUTING.md).**
