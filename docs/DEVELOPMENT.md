# RustyDB Development Guide

**Last Updated**: 2025-12-09
**Target Audience**: Contributors and developers

## Table of Contents

1. [Getting Started](#getting-started)
2. [Development Environment](#development-environment)
3. [Project Structure](#project-structure)
4. [Building and Testing](#building-and-testing)
5. [Code Style and Conventions](#code-style-and-conventions)
6. [Module Development Guidelines](#module-development-guidelines)
7. [Testing Strategy](#testing-strategy)
8. [Performance Considerations](#performance-considerations)
9. [Security Guidelines](#security-guidelines)
10. [Debugging Tips](#debugging-tips)
11. [Contributing](#contributing)
12. [Common Issues](#common-issues)

---

## Getting Started

### Prerequisites

**Required**:
- Rust 1.70 or higher (latest stable recommended)
- Cargo (comes with Rust)
- Git

**Optional**:
- rust-analyzer (LSP for IDE support)
- cargo-watch (auto-recompile on changes)
- cargo-audit (security vulnerability scanning)
- cargo-flamegraph (profiling)

### Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/harborgrid-justin/rusty-db.git
cd rusty-db

# Build the project
cargo build

# Run tests to verify setup
cargo test
```

### Quick Start

```bash
# Run the database server
cargo run --bin rusty-db-server

# In another terminal, run the CLI client
cargo run --bin rusty-db-cli

# Run a simple query
rusty-db> CREATE TABLE test (id INT, name VARCHAR(100));
rusty-db> INSERT INTO test VALUES (1, 'Alice');
rusty-db> SELECT * FROM test;
```

---

## Development Environment

### Recommended IDE Setup

**VS Code**:
- Extensions:
  - rust-analyzer
  - Even Better TOML
  - Error Lens
  - CodeLLDB (debugging)
- Settings:
  ```json
  {
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all"
  }
  ```

**IntelliJ IDEA / CLion**:
- Install Rust plugin
- Enable Clippy integration
- Configure formatter on save

### Useful Cargo Commands

```bash
# Check compilation without building
cargo check

# Build in release mode (optimized)
cargo build --release

# Run specific binary
cargo run --bin rusty-db-server

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench

# Format code
cargo fmt

# Run linter
cargo clippy

# Auto-fix lints
cargo clippy --fix

# View documentation
cargo doc --open

# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Check for security vulnerabilities
cargo audit

# Watch for changes and auto-rebuild
cargo watch -x test
```

---

## Project Structure

```
rusty-db/
├── src/                      # Source code
│   ├── lib.rs               # Library entry point
│   ├── main.rs              # Server binary
│   ├── cli.rs               # CLI client binary
│   ├── error.rs             # Error types
│   ├── common.rs            # Shared types and traits
│   │
│   ├── storage/             # Storage layer
│   ├── buffer/              # Buffer pool
│   ├── memory/              # Memory management
│   ├── io/                  # Async I/O
│   │
│   ├── transaction/         # Transaction management
│   ├── parser/              # SQL parsing
│   ├── execution/           # Query execution
│   ├── optimizer_pro/       # Query optimization
│   ├── index/               # Index structures
│   │
│   ├── network/             # Network layer
│   ├── api/                 # REST/GraphQL APIs
│   ├── pool/                # Connection pooling
│   │
│   ├── security/            # Security modules
│   ├── security_vault/      # Advanced security
│   ├── clustering/          # Distributed clustering
│   ├── rac/                 # RAC implementation
│   ├── replication/         # Replication
│   ├── backup/              # Backup & recovery
│   ├── monitoring/          # Monitoring
│   │
│   ├── graph/               # Graph database
│   ├── document_store/      # Document database
│   ├── spatial/             # Geospatial
│   ├── ml/                  # Machine learning
│   ├── inmemory/            # In-memory store
│   │
│   └── [other modules]/     # Additional features
│
├── tests/                   # Integration tests
├── examples/                # Example code
├── benches/                 # Benchmarks
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md      # System architecture
│   ├── DEVELOPMENT.md       # This file
│   ├── SECURITY_ARCHITECTURE.md
│   └── [other docs]/
│
├── .scratchpad/             # Development coordination
├── Cargo.toml               # Project configuration
├── Cargo.lock               # Dependency lock file
├── README.md                # Project overview
└── CLAUDE.md                # AI assistant guidance
```

---

## Building and Testing

### Build Profiles

**Debug Build** (default):
```bash
cargo build
# Fast compilation, no optimizations, includes debug symbols
```

**Release Build**:
```bash
cargo build --release
# Slow compilation, full optimizations, stripped symbols
```

**Profile Customization** (in `Cargo.toml`):
```toml
[profile.dev]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
lto = true        # Link-time optimization
codegen-units = 1 # Better optimization, slower compile
```

### Running Tests

**Unit Tests**:
```bash
# Run all tests
cargo test

# Run specific module tests
cargo test storage::
cargo test transaction::

# Run specific test by name
cargo test test_buffer_pool

# Run with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored

# Run tests in parallel (default)
cargo test

# Run tests serially
cargo test -- --test-threads=1
```

**Integration Tests**:
```bash
# Located in tests/ directory
cargo test --test integration_test_name
```

**Documentation Tests**:
```bash
# Test code examples in doc comments
cargo test --doc
```

### Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench benchmark_name

# Compare benchmarks
cargo bench --bench benchmark_name -- --save-baseline baseline_name
```

---

## Code Style and Conventions

### Rust Style Guidelines

**Follow Rust API Guidelines**: https://rust-lang.github.io/api-guidelines/

**Key Conventions**:
- Use `snake_case` for functions, variables, modules
- Use `PascalCase` for types, traits, enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Maximum line length: 100 characters
- Use 4 spaces for indentation (enforced by rustfmt)

### Code Formatting

```bash
# Format entire project
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Format specific file
rustfmt src/storage/page.rs
```

**Configuration** (`.rustfmt.toml`):
```toml
max_width = 100
hard_tabs = false
tab_spaces = 4
edition = "2021"
```

### Linting with Clippy

```bash
# Run Clippy
cargo clippy

# Fail on warnings
cargo clippy -- -D warnings

# Auto-fix issues
cargo clippy --fix
```

**Common Clippy Lints to Follow**:
- `clippy::pedantic`: Extra pedantic lints
- `clippy::nursery`: Experimental lints
- `clippy::cargo`: Cargo-specific lints

### Naming Conventions

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

### Documentation

**Module-level Documentation**:
```rust
//! Buffer pool management module.
//!
//! This module provides a high-performance buffer pool for caching
//! database pages in memory.
```

**Function Documentation**:
```rust
/// Pins a page in the buffer pool.
///
/// # Arguments
///
/// * `page_id` - The ID of the page to pin
///
/// # Returns
///
/// A pinned page guard that automatically unpins on drop.
///
/// # Errors
///
/// Returns `DbError::PageNotFound` if the page doesn't exist.
///
/// # Examples
///
/// ```
/// let page = buffer_pool.pin_page(page_id)?;
/// // Use page...
/// // Page is automatically unpinned when it goes out of scope
/// ```
pub fn pin_page(&self, page_id: PageId) -> Result<PageGuard> {
    // Implementation
}
```

### Error Handling

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

**Provide context for errors**:
```rust
use anyhow::Context;

let page = self.disk.read(page_id)
    .context(format!("Failed to read page {}", page_id))?;
```

---

## Module Development Guidelines

### Adding a New Module

1. **Create module directory**:
   ```bash
   mkdir src/my_module
   ```

2. **Create `mod.rs`**:
   ```rust
   //! My module for doing X.

   mod core;
   mod types;
   mod operations;

   pub use core::*;
   pub use types::*;
   pub use operations::*;
   ```

3. **Declare in `lib.rs`**:
   ```rust
   pub mod my_module;
   ```

4. **Add tests**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_basic_functionality() {
           // Test code
       }
   }
   ```

### Refactoring Large Files

**Guidelines**:
- Target: <500 lines per file
- Group related functionality
- Preserve public APIs
- Use re-exports in `mod.rs`
- Update documentation

**Example Refactoring**:

Before:
```
src/storage/
└── mod.rs (2000 lines)
```

After:
```
src/storage/
├── mod.rs (re-exports)
├── page.rs (~400 lines)
├── disk.rs (~400 lines)
├── buffer.rs (~400 lines)
└── cache.rs (~400 lines)
```

### Module Dependencies

**Rules**:
1. All modules depend on `error` and `common`
2. Lower layers don't depend on higher layers
3. Avoid circular dependencies
4. Use traits for dependency inversion

**Dependency Graph** (allowed):
```
execution → transaction → storage → buffer → io
    ↓           ↓           ↓         ↓       ↓
  error ← ← ← ← common ← ← ← ← ← ← ← ← ← ← ←
```

---

## Testing Strategy

### Unit Tests

**Location**: Same file as code, in `#[cfg(test)]` module

**Example**:
```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_add_negative() {
        assert_eq!(add(-1, 1), 0);
    }
}
```

### Integration Tests

**Location**: `tests/` directory

**Example** (`tests/storage_test.rs`):
```rust
use rusty_db::{storage::*, buffer::*};

#[test]
fn test_page_write_read() {
    let disk = DiskManager::new("test_db");
    let page = Page::new(1);
    disk.write_page(&page).unwrap();
    let read_page = disk.read_page(1).unwrap();
    assert_eq!(page.id(), read_page.id());
}
```

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
    fn test_pin_page_success() { }

    // Error cases
    #[test]
    fn test_pin_page_not_found() { }

    // Edge cases
    #[test]
    fn test_pin_page_buffer_full() { }

    // Concurrent tests
    #[tokio::test]
    async fn test_concurrent_access() { }
}
```

### Test Coverage

```bash
# Install tarpaulin for coverage
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html

# View coverage report
open tarpaulin-report.html
```

---

## Performance Considerations

### Profiling

**CPU Profiling**:
```bash
# Install flamegraph
cargo install flamegraph

# Run with profiling
cargo flamegraph --bin rusty-db-server

# Opens flamegraph in browser
```

**Memory Profiling**:
```bash
# Using Valgrind
valgrind --tool=massif target/release/rusty-db-server

# Using heaptrack
heaptrack target/release/rusty-db-server
```

### Benchmarking

**Example Benchmark** (`benches/buffer_pool.rs`):
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rusty_db::buffer::BufferPool;

fn benchmark_pin_page(c: &mut Criterion) {
    let pool = BufferPool::new(1000);

    c.bench_function("pin_page", |b| {
        b.iter(|| {
            pool.pin_page(black_box(1))
        })
    });
}

criterion_group!(benches, benchmark_pin_page);
criterion_main!(benches);
```

### Performance Best Practices

1. **Use SIMD where applicable** (enable `simd` feature)
2. **Minimize allocations** (use arena allocators, object pools)
3. **Avoid unnecessary clones** (use references, `Cow`)
4. **Use lock-free data structures** for hot paths
5. **Profile before optimizing** (measure, don't guess)
6. **Benchmark critical paths** (use `criterion`)

---

## Security Guidelines

### Input Validation

```rust
// Always validate user input
pub fn create_table(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_TABLE_NAME_LEN {
        return Err(DbError::InvalidTableName);
    }

    if !is_valid_identifier(name) {
        return Err(DbError::InvalidTableName);
    }

    // Proceed with table creation
    Ok(())
}
```

### SQL Injection Prevention

```rust
// Use parameterized queries
let stmt = "SELECT * FROM users WHERE id = ?";
let result = executor.execute(stmt, &[&user_id])?;

// Never concatenate user input
// BAD: format!("SELECT * FROM users WHERE id = {}", user_id)
```

### Sensitive Data

```rust
// Mark sensitive data
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
        self.password_hash.zeroize();
    }
}
```

### Secure Coding Practices

1. **Never trust user input**
2. **Use prepared statements**
3. **Encrypt sensitive data** at rest and in transit
4. **Validate all external data**
5. **Minimize privileges** (principle of least privilege)
6. **Audit security-critical operations**
7. **Keep dependencies updated** (`cargo audit`)

---

## Debugging Tips

### Logging

```rust
use tracing::{debug, info, warn, error};

// Use structured logging
info!(page_id = ?page_id, "Pinning page");
debug!(transaction_id = txn_id, "Starting transaction");
warn!(connection_count = count, "Approaching connection limit");
error!(error = ?err, "Failed to write page");
```

### Debugging with `lldb`/`gdb`

```bash
# Build with debug symbols
cargo build

# Run with debugger
rust-gdb target/debug/rusty-db-server

# Set breakpoint
(gdb) break buffer_pool.rs:123

# Run
(gdb) run

# Step through
(gdb) step
(gdb) next

# Print variable
(gdb) print page_id
```

### Common Debugging Commands

```bash
# Check compilation errors
cargo check

# Expand macros
cargo expand module::submodule

# Show type of expression
# (in code, temporarily)
let _: () = some_expression;  // Compiler will show type
```

---

## Contributing

### Contribution Workflow

1. **Fork the repository**
2. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-feature
   ```
3. **Make changes**:
   - Write code
   - Add tests
   - Update documentation
4. **Run checks**:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   ```
5. **Commit changes**:
   ```bash
   git add .
   git commit -m "Add feature X"
   ```
6. **Push to fork**:
   ```bash
   git push origin feature/my-feature
   ```
7. **Create pull request**

### Pull Request Guidelines

**PR Title Format**: `[Module] Brief description`
- Examples:
  - `[Storage] Add LSM tree implementation`
  - `[Security] Fix SQL injection vulnerability`
  - `[Docs] Update architecture documentation`

**PR Description**:
```markdown
## Description
Brief description of changes.

## Motivation
Why is this change needed?

## Changes
- Added X
- Modified Y
- Fixed Z

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] Tests pass
- [ ] No new warnings
```

### Code Review Process

**As Author**:
- Respond to all comments
- Make requested changes
- Update PR description if scope changes

**As Reviewer**:
- Check code correctness
- Verify tests are adequate
- Ensure documentation is updated
- Consider performance implications
- Look for security issues

---

## Common Issues

### Build Errors

**Missing dependencies**:
```bash
# Update Cargo.lock
cargo update
```

**Conflicting versions**:
```bash
# Clean and rebuild
cargo clean
cargo build
```

### Test Failures

**Flaky tests**:
- Check for race conditions
- Use deterministic randomness (seeded)
- Ensure proper cleanup

**Timeout issues**:
- Increase timeout in test
- Check for deadlocks

### Performance Issues

**Slow compilation**:
- Use `cargo check` instead of `cargo build`
- Reduce generic complexity
- Split large files

**Runtime performance**:
- Profile first (flamegraph)
- Check for allocations (heaptrack)
- Use release mode for benchmarks

### IDE Issues

**rust-analyzer slow**:
- Exclude target directory
- Disable unused features
- Increase memory limit

---

## Resources

### Official Documentation
- Rust Book: https://doc.rust-lang.org/book/
- Rust by Example: https://doc.rust-lang.org/rust-by-example/
- Cargo Book: https://doc.rust-lang.org/cargo/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/

### Database Resources
- CMU Database Systems Course: https://15445.courses.cs.cmu.edu/
- PostgreSQL Internals: https://www.postgresql.org/docs/current/internals.html
- Database Internals (book by Alex Petrov)

### RustyDB Documentation
- Architecture: `docs/ARCHITECTURE.md`
- Security: `docs/SECURITY_ARCHITECTURE.md`
- AI Assistant Guidance: `CLAUDE.md`

---

## Getting Help

- **GitHub Issues**: Report bugs, request features
- **Discussions**: Ask questions, share ideas
- **Discord**: Real-time chat (if available)
- **Email**: maintainers@rustydb.io (if available)

---

*Happy coding! Welcome to the RustyDB project.*
