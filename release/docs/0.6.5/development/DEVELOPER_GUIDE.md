# RustyDB v0.6.5 Developer Guide

**Version**: 0.6.5
**Release Date**: December 2025
**Status**: ✅ Validated for Enterprise Deployment
**Target Audience**: Software Engineers, Database Developers, Contributors

---

## Document Control

| Property | Value |
|----------|-------|
| Document Version | 1.0.0 |
| Last Updated | 2025-12-29 |
| Validation Status | ✅ ENTERPRISE VALIDATED |
| Compliance Level | Production Ready |
| Reviewed By | Enterprise Documentation Agent 8 |

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Development Environment Setup](#development-environment-setup)
4. [Project Architecture](#project-architecture)
5. [Development Workflow](#development-workflow)
6. [Building RustyDB](#building-rustydb)
7. [Testing Strategy](#testing-strategy)
8. [Debugging Techniques](#debugging-techniques)
9. [Performance Profiling](#performance-profiling)
10. [Contributing to RustyDB](#contributing-to-rustydb)
11. [Best Practices](#best-practices)
12. [Troubleshooting](#troubleshooting)

---

## Introduction

### About RustyDB v0.6.5

RustyDB is an **enterprise-grade, Oracle-compatible database management system** written in Rust, designed for high-performance, scalability, and security. Version 0.6.5 represents the culmination of extensive development and testing, featuring:

- **60+ Specialized Modules**: Comprehensive database functionality
- **Enterprise Security**: 10+ security modules with advanced threat protection
- **Multi-Model Support**: Relational, Graph, Document, Spatial, Time-Series
- **Distributed Architecture**: RAC, clustering, replication, sharding
- **Advanced Query Optimization**: Cost-based optimizer with adaptive execution
- **MVCC Transaction Management**: Full ACID compliance with multiple isolation levels

### Why Develop for RustyDB?

- **Memory Safety**: Rust's ownership model eliminates entire classes of bugs
- **Performance**: Zero-cost abstractions and native code compilation
- **Concurrency**: Safe, fearless concurrency without data races
- **Type Safety**: Compile-time guarantees reduce runtime errors
- **Modern Tooling**: Cargo, rustfmt, clippy, and excellent IDE support
- **Growing Ecosystem**: Leverage the extensive Rust crate ecosystem

### Prerequisites

**Required Knowledge**:
- Rust programming language (intermediate level)
- Database concepts (ACID, transactions, indexes)
- SQL (advanced knowledge recommended)
- Git version control
- Unix/Linux command line

**Required Software**:
- Rust 1.70+ (stable toolchain)
- Git 2.30+
- 8GB+ RAM (16GB recommended)
- 10GB+ disk space

**Recommended Knowledge**:
- Distributed systems
- Lock-free data structures
- Query optimization
- Storage engines

---

## Getting Started

### Quick Start (5 Minutes)

```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 2. Clone repository
git clone https://github.com/harborgrid-justin/rusty-db.git
cd rusty-db

# 3. Build the project
cargo build --release

# 4. Run tests
cargo test

# 5. Start the database server
cargo run --release --bin rusty-db-server

# 6. In another terminal, start the CLI client
cargo run --bin rusty-db-cli
```

### First Steps After Installation

1. **Verify Installation**:
   ```bash
   cargo --version  # Should show 1.70+
   rustc --version  # Should show 1.70+
   ```

2. **Run Quick Tests**:
   ```bash
   cargo test storage::
   cargo test transaction::
   ```

3. **Check Code Quality**:
   ```bash
   cargo clippy
   cargo fmt --check
   ```

4. **Generate Documentation**:
   ```bash
   cargo doc --open
   ```

---

## Development Environment Setup

### Recommended IDEs

#### Visual Studio Code (Recommended)

**Extensions**:
- **rust-analyzer**: LSP for Rust (essential)
- **CodeLLDB**: Debugging support
- **Even Better TOML**: TOML syntax highlighting
- **Error Lens**: Inline error highlighting
- **GitLens**: Advanced Git integration
- **Better Comments**: Enhanced comment highlighting

**VS Code Settings** (`.vscode/settings.json`):
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.inlayHints.parameterHints.enable": true,
  "rust-analyzer.inlayHints.typeHints.enable": true,
  "editor.formatOnSave": true,
  "editor.rulers": [100],
  "files.watcherExclude": {
    "**/target/**": true
  }
}
```

#### IntelliJ IDEA / CLion

**Plugins**:
- Rust Plugin (official JetBrains)
- TOML Plugin

**Configuration**:
- Enable "Run Clippy on save"
- Configure rustfmt as external tool
- Set up run configurations for binaries

#### Vim/Neovim

**Plugins**:
- rust.vim or rust-tools.nvim
- coc-rust-analyzer or native LSP
- ale or syntastic for linting

### Additional Development Tools

```bash
# Install useful cargo extensions
cargo install cargo-watch      # Auto-rebuild on file changes
cargo install cargo-edit        # Add/remove dependencies easily
cargo install cargo-audit       # Security vulnerability scanner
cargo install cargo-flamegraph  # CPU profiling
cargo install cargo-tree        # Dependency tree visualization
cargo install cargo-outdated    # Check for outdated dependencies
cargo install cargo-expand      # Macro expansion viewer
```

### Environment Configuration

**Shell Configuration** (~/.bashrc or ~/.zshrc):
```bash
# Rust environment
source "$HOME/.cargo/env"

# Increase stack size for large compilations
export RUST_MIN_STACK=8388608

# Enable incremental compilation (faster rebuilds)
export CARGO_INCREMENTAL=1

# Use faster linker (if available)
# On Linux: install 'mold' and add to .cargo/config.toml
# On macOS: install 'zld'
```

**Cargo Configuration** (~/.cargo/config.toml):
```toml
[build]
# Use all available CPU cores
jobs = 8

[target.x86_64-unknown-linux-gnu]
# Use faster linker (Linux)
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.x86_64-apple-darwin]
# Use faster linker (macOS)
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
```

---

## Project Architecture

### High-Level Architecture

RustyDB is organized into **layered modules** with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────┐
│           Application Layer                              │
│  (CLI Client, Server, API Endpoints)                    │
└─────────────────────────────────────────────────────────┘
                        │
┌─────────────────────────────────────────────────────────┐
│           Query Processing Layer                         │
│  (Parser, Optimizer, Executor, Planner)                 │
└─────────────────────────────────────────────────────────┘
                        │
┌─────────────────────────────────────────────────────────┐
│           Transaction Layer                              │
│  (MVCC, Lock Manager, WAL, Recovery)                    │
└─────────────────────────────────────────────────────────┘
                        │
┌─────────────────────────────────────────────────────────┐
│           Storage Layer                                  │
│  (Buffer Pool, Page Manager, Disk I/O)                  │
└─────────────────────────────────────────────────────────┘
                        │
┌─────────────────────────────────────────────────────────┐
│           Foundation Layer                               │
│  (Error Handling, Common Types, Traits)                 │
└─────────────────────────────────────────────────────────┘
```

### Core Modules Overview

| Module | Purpose | Key Files |
|--------|---------|-----------|
| **error** | Unified error handling | `error.rs` |
| **common** | Shared types and traits | `common.rs` |
| **storage** | Page-based storage engine | `storage/mod.rs` |
| **buffer** | Buffer pool management | `buffer/manager.rs` |
| **transaction** | MVCC transaction engine | `transaction/mod.rs` |
| **parser** | SQL parsing | `parser/mod.rs` |
| **execution** | Query execution | `execution/executor.rs` |
| **optimizer_pro** | Query optimization | `optimizer_pro/cost_model.rs` |
| **index** | Index structures (B-Tree, LSM, etc.) | `index/mod.rs` |
| **network** | Network protocol | `network/mod.rs` |
| **api** | REST/GraphQL APIs | `api/mod.rs` |
| **security** | Security modules | `security/mod.rs` |
| **clustering** | Distributed clustering | `clustering/mod.rs` |
| **replication** | Database replication | `replication/mod.rs` |
| **backup** | Backup and recovery | `backup/mod.rs` |
| **monitoring** | System monitoring | `monitoring/mod.rs` |

See [MODULE_REFERENCE.md](./MODULE_REFERENCE.md) for complete module documentation.

### Module Dependency Rules

1. **Foundational modules** (`error`, `common`) have no internal dependencies
2. **Lower layers** (storage, buffer) don't depend on higher layers (execution, API)
3. **No circular dependencies** are allowed
4. **Use traits** for dependency inversion when needed

**Dependency Flow**:
```
api → execution → transaction → storage → buffer → io
 ↓        ↓           ↓           ↓         ↓      ↓
error ← ← ← ← ← ← common ← ← ← ← ← ← ← ← ← ← ← ←
```

---

## Development Workflow

### Daily Development Cycle

```bash
# 1. Update your local repository
git pull origin main

# 2. Create a feature branch
git checkout -b feature/my-feature

# 3. Make changes to code

# 4. Run quick checks (during development)
cargo check                    # Fast compilation check
cargo watch -x check           # Auto-check on file changes

# 5. Run tests for affected modules
cargo test storage::           # Test specific module
cargo test -- --nocapture      # See test output

# 6. Format code
cargo fmt

# 7. Run linter
cargo clippy --fix             # Auto-fix lints
cargo clippy -- -D warnings    # Fail on warnings

# 8. Run full test suite before commit
cargo test

# 9. Commit changes
git add .
git commit -m "feat: add new storage optimization"

# 10. Push to your fork
git push origin feature/my-feature
```

### Cargo Commands Reference

| Command | Purpose | When to Use |
|---------|---------|-------------|
| `cargo check` | Fast compilation check | During active development |
| `cargo build` | Build debug binary | Testing locally |
| `cargo build --release` | Build optimized binary | Performance testing |
| `cargo test` | Run all tests | Before commit |
| `cargo test <module>::` | Test specific module | Module development |
| `cargo test -- --nocapture` | Show test output | Debugging tests |
| `cargo bench` | Run benchmarks | Performance validation |
| `cargo fmt` | Format code | Before commit (always) |
| `cargo clippy` | Run linter | Before commit (always) |
| `cargo doc --open` | Generate docs | Understanding APIs |
| `cargo clean` | Remove build artifacts | Troubleshooting builds |
| `cargo update` | Update dependencies | Monthly maintenance |
| `cargo audit` | Security scan | Before releases |

### Git Workflow

**Branch Naming Convention**:
- `feature/description` - New features
- `fix/description` - Bug fixes
- `refactor/description` - Code refactoring
- `docs/description` - Documentation updates
- `test/description` - Test improvements

**Commit Message Format**:
```
<type>: <subject>

<body>

<footer>
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `test`: Test additions/changes
- `docs`: Documentation changes
- `chore`: Build/tooling changes

**Example**:
```
feat: add LSM tree compaction strategy

Implement tiered compaction strategy for LSM trees to improve
read performance and reduce space amplification.

- Add CompactionStrategy trait
- Implement TieredCompaction
- Add benchmarks for compaction performance

Closes #123
```

---

## Building RustyDB

See [BUILD_INSTRUCTIONS.md](./BUILD_INSTRUCTIONS.md) for detailed build instructions.

### Quick Build Commands

```bash
# Debug build (fast compilation, slower runtime)
cargo build

# Release build (slow compilation, fast runtime)
cargo build --release

# Build specific binary
cargo build --bin rusty-db-server --release
cargo build --bin rusty-db-cli

# Build with specific features
cargo build --features "simd,io_uring"

# Build documentation
cargo doc --no-deps --open
```

### Build Profiles

**Development Profile** (default):
- No optimizations (opt-level = 0)
- Debug symbols included
- Fast compilation
- Assertions enabled

**Release Profile**:
- Full optimizations (opt-level = 3)
- Debug symbols stripped
- Link-time optimization (LTO)
- Slower compilation, faster runtime

**Custom Profile** (in Cargo.toml):
```toml
[profile.profiling]
inherits = "release"
debug = true
```

---

## Testing Strategy

See [TESTING_GUIDE.md](./TESTING_GUIDE.md) for comprehensive testing documentation.

### Test Categories

1. **Unit Tests**: Test individual functions/methods
2. **Integration Tests**: Test module interactions
3. **Functional Tests**: Test end-to-end functionality
4. **Performance Tests**: Benchmarks
5. **Security Tests**: Security validation

### Running Tests

```bash
# All tests
cargo test

# Specific module
cargo test storage::

# Specific test
cargo test test_buffer_pool

# With output
cargo test -- --nocapture

# Single-threaded (for debugging)
cargo test -- --test-threads=1

# Ignored tests
cargo test -- --ignored

# Documentation tests
cargo test --doc
```

### Writing Tests

**Unit Test Example**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_creation() {
        let page = Page::new(1);
        assert_eq!(page.id(), 1);
        assert_eq!(page.size(), PAGE_SIZE);
    }

    #[test]
    fn test_page_write_read() -> Result<()> {
        let mut page = Page::new(1);
        let data = b"Hello, RustyDB!";
        page.write(0, data)?;

        let mut buffer = vec![0u8; data.len()];
        page.read(0, &mut buffer)?;

        assert_eq!(&buffer, data);
        Ok(())
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn test_page_write_overflow() {
        let mut page = Page::new(1);
        let data = vec![0u8; PAGE_SIZE + 1];
        page.write(0, &data).unwrap();
    }
}
```

---

## Debugging Techniques

### Print Debugging

```rust
// Simple debug print
println!("Value: {:?}", value);

// Pretty print
println!("{:#?}", complex_struct);

// Conditional debug
#[cfg(debug_assertions)]
println!("Debug mode value: {:?}", value);
```

### Using the Debugger

**LLDB/GDB**:
```bash
# Build with debug symbols
cargo build

# Run with debugger
rust-lldb target/debug/rusty-db-server
# or
rust-gdb target/debug/rusty-db-server

# Set breakpoint
(lldb) breakpoint set -f buffer.rs -l 123
(lldb) b buffer::BufferPool::pin_page

# Run
(lldb) run

# Step through
(lldb) step
(lldb) next
(lldb) continue

# Inspect variables
(lldb) print page_id
(lldb) frame variable
```

**VS Code Debugging**:

`.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug RustyDB Server",
      "cargo": {
        "args": [
          "build",
          "--bin=rusty-db-server"
        ]
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### Logging

```rust
use tracing::{debug, info, warn, error};

// Structured logging
info!(page_id = ?page_id, "Pinning page");
debug!(transaction_id = txn_id, "Starting transaction");
warn!(connection_count = count, "High connection count");
error!(error = ?err, "Failed to write page");
```

### Common Debug Scenarios

**Memory Issues**:
```bash
# Run with Valgrind
valgrind --leak-check=full target/debug/rusty-db-server

# Run with AddressSanitizer
RUSTFLAGS="-Z sanitizer=address" cargo run
```

**Performance Issues**:
```bash
# CPU profiling
cargo flamegraph --bin rusty-db-server

# Memory profiling
heaptrack target/release/rusty-db-server
```

---

## Performance Profiling

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile your application
cargo flamegraph --bin rusty-db-server

# Profile with specific workload
cargo flamegraph --bin rusty-db-server -- --config test.toml

# Profile tests
cargo flamegraph --test integration_tests
```

### Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench buffer_pool

# Save baseline
cargo bench -- --save-baseline main

# Compare to baseline
git checkout feature/optimization
cargo bench -- --baseline main
```

**Benchmark Example**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_pin_page(c: &mut Criterion) {
    let pool = BufferPool::new(1000);

    c.bench_function("pin_page", |b| {
        b.iter(|| {
            let _guard = pool.pin_page(black_box(1)).unwrap();
        });
    });
}

criterion_group!(benches, benchmark_pin_page);
criterion_main!(benches);
```

### Memory Profiling

```bash
# Install heaptrack
# Ubuntu: sudo apt install heaptrack
# macOS: brew install heaptrack

# Profile memory usage
heaptrack target/release/rusty-db-server

# Analyze results
heaptrack_gui heaptrack.rusty-db-server.*.gz
```

---

## Contributing to RustyDB

See [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed contribution guidelines.

### Quick Contribution Checklist

- [ ] Code follows Rust style guidelines
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] Commit messages follow convention
- [ ] PR description is clear and complete

---

## Best Practices

### Error Handling

```rust
// ✅ Good: Use ? operator
pub fn read_page(page_id: PageId) -> Result<Page> {
    let data = self.disk.read(page_id)?;
    let page = Page::deserialize(&data)?;
    Ok(page)
}

// ❌ Bad: Nested match statements
pub fn read_page(page_id: PageId) -> Result<Page> {
    match self.disk.read(page_id) {
        Ok(data) => match Page::deserialize(&data) {
            Ok(page) => Ok(page),
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    }
}

// ✅ Good: Add context to errors
use anyhow::Context;

let page = self.disk.read(page_id)
    .context(format!("Failed to read page {}", page_id))?;
```

### Performance Tips

1. **Avoid Unnecessary Allocations**:
   ```rust
   // ✅ Good: Use references
   fn process_data(data: &[u8]) { }

   // ❌ Bad: Unnecessary clone
   fn process_data(data: Vec<u8>) { }
   ```

2. **Use Appropriate Collections**:
   ```rust
   // ✅ Good: Use HashMap for lookups
   use std::collections::HashMap;
   let mut cache: HashMap<PageId, Page> = HashMap::new();

   // ❌ Bad: Linear search in Vec
   let mut cache: Vec<(PageId, Page)> = Vec::new();
   ```

3. **Leverage SIMD** (when enabled):
   ```rust
   #[cfg(feature = "simd")]
   use crate::simd::vectorized_sum;

   #[cfg(not(feature = "simd"))]
   fn vectorized_sum(data: &[i32]) -> i32 {
       data.iter().sum()
   }
   ```

### Code Style

See [CODING_STANDARDS.md](./CODING_STANDARDS.md) for comprehensive style guide.

**Key Points**:
- Follow Rust API Guidelines
- Use `rustfmt` for consistent formatting
- Maximum line length: 100 characters
- Use meaningful variable names
- Document public APIs with `///` comments
- Avoid `unsafe` unless absolutely necessary

---

## Troubleshooting

### Common Issues

**Issue**: Compilation is very slow
```bash
# Solution 1: Use faster linker
# Add to .cargo/config.toml (see Environment Configuration)

# Solution 2: Reduce parallelism
cargo build -j 4

# Solution 3: Clean build artifacts
cargo clean
```

**Issue**: Tests are failing sporadically
```bash
# Solution: Run tests single-threaded
cargo test -- --test-threads=1

# Check for race conditions in tests
```

**Issue**: Out of memory during compilation
```bash
# Solution: Reduce parallel jobs
cargo build -j 2

# Or build in release mode (uses less memory for intermediate artifacts)
cargo build --release
```

**Issue**: rust-analyzer is slow/unresponsive
```bash
# Solution 1: Exclude target directory (VS Code settings.json)
"files.watcherExclude": {
  "**/target/**": true
}

# Solution 2: Restart rust-analyzer
# VS Code: Cmd/Ctrl + Shift + P → "rust-analyzer: Restart server"
```

---

## Additional Resources

### Documentation
- [Architecture Documentation](../../architecture/ARCHITECTURE.md)
- [API Documentation](../../api/API_REFERENCE.md)
- [Security Documentation](../../security/SECURITY_ARCHITECTURE.md)
- [Deployment Guide](../../deployment/DEPLOYMENT_GUIDE.md)

### External Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Database Internals (Book)](https://www.databass.dev/)
- [CMU Database Systems Course](https://15445.courses.cs.cmu.edu/)

### Getting Help
- GitHub Issues: Report bugs and request features
- GitHub Discussions: Ask questions and share ideas
- Documentation: Check docs/ directory
- CLAUDE.md: AI assistant guidance for Claude Code

---

**Document Status**: ✅ Enterprise Validated for Production Use
**Last Validation**: 2025-12-29
**Next Review**: 2026-03-29
